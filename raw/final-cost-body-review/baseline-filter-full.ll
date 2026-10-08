define void @purrdf_sparql_eval::expr::eval_filter::<purrdf_core::ir::dataset::RdfDataset>(ptr dead_on_unwind noalias nofree noundef writable writeonly sret([112 x i8]) align 16 captures(none) dereferenceable(112) %0, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(144) %1, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(64) %2, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(144) %3, ptr noalias nofree noundef align 16 dereferenceable(1248) %4) unnamed_addr #10 personality ptr @rust_eh_personality !guid !33902 {
  %6 = alloca [96 x i8], align 16
  %7 = alloca [96 x i8], align 16
  %8 = alloca [96 x i8], align 16
  %9 = alloca [96 x i8], align 16
  %10 = alloca [24 x i8], align 8
  %11 = alloca [24 x i8], align 8
  %12 = alloca [48 x i8], align 8
  %13 = alloca [32 x i8], align 8
  %14 = alloca [8 x i8], align 8
  %15 = alloca [24 x i8], align 8
  %16 = alloca [8 x i8], align 8
  %17 = alloca [24 x i8], align 8
  %18 = alloca [24 x i8], align 8
  %19 = alloca [32 x i8], align 8
  %20 = alloca [8 x i8], align 8
  %21 = alloca [16 x i8], align 8
  %22 = alloca [248 x i8], align 8
  %23 = alloca [24 x i8], align 8
  %24 = alloca [32 x i8], align 8
  %25 = alloca [8 x i8], align 8
  %26 = alloca [16 x i8], align 8
  %27 = alloca [32 x i8], align 8
  %28 = alloca [24 x i8], align 8
  %29 = alloca [48 x i8], align 8
  %30 = alloca [8 x i8], align 8
  %31 = alloca [24 x i8], align 8
  %32 = alloca [24 x i8], align 8
  %33 = alloca [24 x i8], align 8
  %34 = alloca [24 x i8], align 8
  %35 = alloca [32 x i8], align 8
  %36 = alloca [8 x i8], align 8
  %37 = alloca [40 x i8], align 8
  %38 = alloca [24 x i8], align 8
  %39 = alloca [48 x i8], align 8
  %40 = alloca [24 x i8], align 8
  %41 = alloca [24 x i8], align 8
  %42 = alloca [23 x i8], align 1
  %43 = alloca [24 x i8], align 8
  %44 = alloca [80 x i8], align 8
  %45 = alloca [24 x i8], align 8
  %46 = alloca [24 x i8], align 8
  %47 = alloca [80 x i8], align 8
  %48 = alloca [80 x i8], align 8
  %49 = alloca [24 x i8], align 8
  %50 = alloca [80 x i8], align 8
  %51 = alloca [80 x i8], align 8
  %52 = alloca [24 x i8], align 8
  %53 = alloca [24 x i8], align 8
  %54 = alloca [80 x i8], align 8
  %55 = alloca [24 x i8], align 8
  %56 = alloca [24 x i8], align 8
  %57 = alloca [32 x i8], align 8
  %58 = alloca [8 x i8], align 8
  %59 = alloca [8 x i8], align 8
  %60 = alloca [16 x i8], align 8
  %61 = alloca [40 x i8], align 8
  %62 = alloca [24 x i8], align 8
  %63 = alloca [24 x i8], align 8
  %64 = alloca [24 x i8], align 8
  %65 = alloca [24 x i8], align 8
  %66 = alloca [24 x i8], align 8
  %67 = alloca [24 x i8], align 8
  %68 = alloca [24 x i8], align 8
  %69 = alloca [32 x i8], align 8
  %70 = alloca [160 x i8], align 8
  %71 = alloca [32 x i8], align 8
  %72 = alloca [8 x i8], align 8
  %73 = alloca [24 x i8], align 8
  %74 = alloca [32 x i8], align 8
  %75 = alloca [32 x i8], align 8
  %76 = alloca [24 x i8], align 8
  %77 = alloca [96 x i8], align 16
  %78 = alloca [24 x i8], align 8
  %79 = alloca [24 x i8], align 8
  %80 = alloca [160 x i8], align 8
  %81 = alloca [32 x i8], align 8
  %82 = alloca [24 x i8], align 8
  %83 = alloca [24 x i8], align 8
  %84 = alloca [32 x i8], align 8
  %85 = alloca [24 x i8], align 8
  %86 = alloca [48 x i8], align 1
  %87 = alloca [24 x i8], align 8
  %88 = alloca [224 x i8], align 8
  %89 = alloca [24 x i8], align 8
  %90 = alloca [112 x i8], align 16
  %91 = alloca [32 x i8], align 8
  %92 = alloca [104 x i8], align 8
  %93 = alloca [96 x i8], align 8
  %94 = alloca [96 x i8], align 8
  %95 = alloca [24 x i8], align 8
  %96 = alloca [24 x i8], align 8
  %97 = alloca [80 x i8], align 16
  %98 = alloca [24 x i8], align 8
  %99 = alloca [40 x i8], align 8
  %100 = alloca [32 x i8], align 8
  %101 = alloca [32 x i8], align 8
  %102 = alloca [24 x i8], align 8
  %103 = alloca [16 x i8], align 8
  %104 = alloca [80 x i8], align 16
  %105 = alloca [24 x i8], align 8
  %106 = alloca [200 x i8], align 8
  %107 = alloca [24 x i8], align 8
  %108 = alloca [24 x i8], align 8
  %109 = alloca [24 x i8], align 8
  %110 = alloca [24 x i8], align 8
  %111 = alloca [248 x i8], align 8
  %112 = alloca [240 x i8], align 8
  %113 = alloca [208 x i8], align 8
  %114 = alloca [32 x i8], align 8
  %115 = alloca [24 x i8], align 8
  %116 = alloca [32 x i8], align 8
  %117 = alloca [256 x i8], align 16
  %118 = alloca [208 x i8], align 16
  %119 = alloca [24 x i8], align 8
  %120 = alloca [8 x i8], align 8
  %121 = alloca [24 x i8], align 8
  %122 = alloca [216 x i8], align 8
  %123 = alloca [200 x i8], align 8
  %124 = alloca [8 x i8], align 8
  %125 = alloca [104 x i8], align 8
  %126 = alloca [32 x i8], align 8
  %127 = alloca [32 x i8], align 8
  %128 = alloca [104 x i8], align 8
; call <purrdf_sparql_eval::governor::lift::Lift>::at
  call void @<purrdf_sparql_eval::governor::lift::Lift>::at(ptr noalias nofree noundef nonnull sret([104 x i8]) align 8 captures(none) dereferenceable(104) %128, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %1)
  call void @llvm.lifetime.start.p0(ptr nonnull %126)
  call void @llvm.lifetime.start.p0(ptr nonnull %125)
; invoke purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
  invoke fastcc void @purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>(ptr noalias nofree noundef nonnull align 16 captures(none) dereferenceable(112) %90, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %3, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %4, ptr noundef nonnull readonly align 8 dereferenceable(144) %3)
          to label %132 unwind label %130, !inline_history !24788

129:                                              ; preds = %3188
  br i1 %3001, label %3193, label %3191

130:                                              ; preds = %138, %5
  %131 = landingpad { ptr, i32 }
          cleanup
  br label %3193

132:                                              ; preds = %5
  %133 = load i64, ptr %90, align 16, !range !1855, !noundef !1708
  %134 = trunc nuw i64 %133 to i1
  br i1 %134, label %135, label %138

135:                                              ; preds = %132
  %136 = getelementptr inbounds nuw i8, ptr %90, i64 16
  %137 = getelementptr inbounds nuw i8, ptr %0, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(96) %137, ptr noundef nonnull align 16 dereferenceable(96) %136, i64 96, i1 false)
  store i64 1, ptr %0, align 16
  call void @llvm.lifetime.end.p0(ptr nonnull %125)
  call void @llvm.lifetime.end.p0(ptr nonnull %126)
  br label %3099

138:                                              ; preds = %132
  %139 = getelementptr inbounds nuw i8, ptr %90, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(96) %125, ptr noundef nonnull align 8 dereferenceable(96) %139, i64 96, i1 false)
; invoke <purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
  invoke fastcc void @<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>(ptr noalias nofree noundef align 8 captures(none) dereferenceable(32) %126, ptr noalias nofree noundef align 8 dereferenceable(104) %128, i64 noundef 0, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(96) %125)
          to label %140 unwind label %130

140:                                              ; preds = %138
  %141 = load i64, ptr %126, align 8, !range !2062, !noundef !1708
  %142 = icmp eq i64 %141, -1
  br i1 %142, label %148, label %143

143:                                              ; preds = %140
  call void @llvm.lifetime.start.p0(ptr nonnull %127)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %127, ptr noundef nonnull align 8 dereferenceable(32) %126, i64 32, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %125)
  call void @llvm.lifetime.end.p0(ptr nonnull %126)
  call void @llvm.lifetime.start.p0(ptr nonnull %124)
  %144 = getelementptr inbounds nuw i8, ptr %127, i64 24
  %145 = load ptr, ptr %144, align 8, !nonnull !1708, !noundef !1708
  %146 = atomicrmw add ptr %145, i64 1 monotonic, align 8
  %147 = icmp slt i64 %146, 0
  br i1 %147, label %152, label %150

148:                                              ; preds = %140
  call void @llvm.lifetime.end.p0(ptr nonnull %125)
  call void @llvm.lifetime.end.p0(ptr nonnull %126)
  %149 = getelementptr inbounds nuw i8, ptr %0, i64 8
; call <purrdf_sparql_eval::governor::lift::Lift>::withheld::<purrdf_core::ir::term::TermId>
  call fastcc void @<purrdf_sparql_eval::governor::lift::Lift>::withheld::<purrdf_core::ir::term::TermId>(ptr noalias nofree noundef align 8 captures(address) dereferenceable(96) %149, ptr noalias nofree noundef align 8 captures(address) dereferenceable(104) %128)
  store i64 0, ptr %0, align 16
  br label %2977

150:                                              ; preds = %143
  store ptr %145, ptr %124, align 8
; invoke <purrdf_sparql_eval::eval::EvalCtx>::may_fork_row_loop
  %151 = invoke fastcc noundef zeroext i1 @<purrdf_sparql_eval::eval::EvalCtx>::may_fork_row_loop(ptr noundef nonnull align 16 %4, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(64) %2)
          to label %159 unwind label %154

152:                                              ; preds = %143
  tail call void @llvm.trap()
  unreachable

153:                                              ; preds = %187
  br i1 %189, label %3181, label %2999

154:                                              ; preds = %2993, %180, %150
  %155 = phi i8 [ 1, %150 ], [ 1, %180 ], [ %2883, %2993 ]
  %156 = landingpad { ptr, i32 }
          cleanup
  br label %3181

157:                                              ; preds = %2886
  %158 = landingpad { ptr, i32 }
          cleanup
  br label %2999

159:                                              ; preds = %150
  %160 = getelementptr inbounds nuw i8, ptr %4, i64 472
  %161 = load i8, ptr %160, align 8, !range !3634
  %162 = icmp eq i8 %161, 2
  %163 = select i1 %151, i1 %162, i1 false
  br i1 %163, label %164, label %180

164:                                              ; preds = %159
  %165 = getelementptr inbounds nuw i8, ptr %4, i64 616
  %166 = load ptr, ptr %165, align 8, !noundef !1708
  %167 = icmp eq ptr %166, null
  br i1 %167, label %180, label %168

168:                                              ; preds = %164
  %169 = getelementptr inbounds nuw i8, ptr %166, i64 24
  %170 = load i64, ptr %169, align 8, !noalias !33903
  %171 = getelementptr inbounds nuw i8, ptr %166, i64 48
  %172 = icmp ult i64 %170, -2
  br i1 %172, label %180, label %173

173:                                              ; preds = %168
  %174 = getelementptr inbounds nuw i8, ptr %166, i64 32
  %175 = load i64, ptr %174, align 8, !noalias !33903
  %176 = icmp ult i64 %175, -2
  br i1 %176, label %180, label %177

177:                                              ; preds = %173
  %178 = load i64, ptr %171, align 8, !noalias !33903
  %179 = icmp ugt i64 %178, -3
  br label %180

180:                                              ; preds = %177, %173, %168, %164, %159
  %181 = phi i1 [ false, %159 ], [ false, %168 ], [ true, %164 ], [ false, %173 ], [ %179, %177 ]
  call void @llvm.lifetime.start.p0(ptr nonnull %123)
  %182 = getelementptr inbounds nuw i8, ptr %127, i64 16
  %183 = load i64, ptr %182, align 8, !noundef !1708
  %184 = icmp ult i64 %183, 230584300921369396
  tail call void @llvm.assume(i1 %184)
; invoke <purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::for_rows::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::for_rows::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef align 8 captures(none) dereferenceable(200) %123, ptr noundef nonnull align 16 %4, i1 noundef zeroext %181, i64 noundef %183)
          to label %185 unwind label %154

185:                                              ; preds = %180
; invoke purrdf_sparql_eval::vm::program_at::<purrdf_core::ir::dataset::RdfDataset>
  %186 = invoke fastcc noundef nonnull ptr @purrdf_sparql_eval::vm::program_at::<purrdf_core::ir::dataset::RdfDataset>(ptr noundef nonnull align 16 %4, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(144) %1, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(64) %2)
          to label %195 unwind label %191

187:                                              ; preds = %524, %191
  %188 = phi i8 [ %192, %191 ], [ %525, %524 ]
  %189 = phi i1 [ %193, %191 ], [ %526, %524 ]
  %190 = phi { ptr, i32 } [ %194, %191 ], [ %527, %524 ]
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(200) %123)
          to label %153 unwind label %545

191:                                              ; preds = %2884, %2882, %195, %185
  %192 = phi i8 [ %2883, %2882 ], [ %534, %2884 ], [ 1, %195 ], [ 1, %185 ]
  %193 = phi i1 [ true, %2882 ], [ false, %2884 ], [ true, %195 ], [ true, %185 ]
  %194 = landingpad { ptr, i32 }
          cleanup
  br label %187

195:                                              ; preds = %185
  call void @llvm.lifetime.start.p0(ptr nonnull %122)
  %196 = load ptr, ptr %124, align 8, !nonnull !1708, !noundef !1708
  %197 = getelementptr inbounds nuw i8, ptr %196, i64 16
; invoke <purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::link::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::link::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef align 8 captures(address) dereferenceable(216) %122, ptr noundef nonnull %186, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(64) %2, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(56) %197, ptr noalias nofree noundef align 16 dereferenceable(1248) %4)
          to label %198 unwind label %191

198:                                              ; preds = %195
  call void @llvm.lifetime.start.p0(ptr nonnull %121)
  br i1 %181, label %547, label %199

199:                                              ; preds = %198
  call void @llvm.lifetime.start.p0(ptr nonnull %102)
  store i64 0, ptr %102, align 8
  %200 = getelementptr inbounds nuw i8, ptr %102, i64 8
  store ptr inttoptr (i64 8 to ptr), ptr %200, align 8
  %201 = getelementptr inbounds nuw i8, ptr %102, i64 16
  store i64 0, ptr %201, align 8
  %202 = getelementptr inbounds nuw i8, ptr %127, i64 8
  %203 = load ptr, ptr %202, align 8, !nonnull !1708, !noundef !1708
  %204 = load i64, ptr %127, align 8, !range !1817, !noundef !1708
  %205 = mul nuw nsw i64 %183, 40
  %206 = getelementptr inbounds nuw i8, ptr %203, i64 %205
  call void @llvm.lifetime.start.p0(ptr nonnull %101)
  store ptr %203, ptr %101, align 8
  %207 = getelementptr inbounds nuw i8, ptr %101, i64 8
  %208 = getelementptr inbounds nuw i8, ptr %101, i64 16
  store i64 %204, ptr %208, align 8
  %209 = getelementptr inbounds nuw i8, ptr %101, i64 24
  store ptr %206, ptr %209, align 8
  call void @llvm.lifetime.start.p0(ptr nonnull %100)
  %210 = icmp eq i64 %183, 0
  br i1 %210, label %.loopexit294, label %211

211:                                              ; preds = %199
  %212 = getelementptr inbounds nuw i8, ptr %99, i64 8
  %213 = getelementptr inbounds nuw i8, ptr %99, i64 16
  %214 = getelementptr inbounds nuw i8, ptr %100, i64 8
  %215 = getelementptr inbounds nuw i8, ptr %9, i64 8
  %216 = getelementptr inbounds nuw i8, ptr %9, i64 12
  %217 = getelementptr inbounds nuw i8, ptr %8, i64 8
  %218 = tail call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %219 = tail call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  br label %227

220:                                              ; preds = %543, %419
  %221 = phi ptr [ %544, %543 ], [ %324, %419 ]
  %222 = phi { ptr, i32 } [ %541, %543 ], [ %417, %419 ]
  %223 = shl i64 %233, 3
  %224 = add i64 %223, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %221, i64 noundef %224, i64 noundef range(i64 1, -9223372036854775807) 4) #88, !noalias !1708
  br label %225

225:                                              ; preds = %540, %416, %220
  %226 = phi { ptr, i32 } [ %417, %416 ], [ %541, %540 ], [ %222, %220 ]
; call core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(32) %101) #89
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %102) #89
  br label %524

227:                                              ; preds = %420, %211
  %228 = phi ptr [ inttoptr (i64 8 to ptr), %211 ], [ %421, %420 ]
  %229 = phi i64 [ 0, %211 ], [ %422, %420 ]
  %230 = phi ptr [ inttoptr (i64 8 to ptr), %211 ], [ %423, %420 ]
  %231 = phi ptr [ %203, %211 ], [ %232, %420 ]
  %232 = getelementptr inbounds nuw i8, ptr %231, i64 40
  %233 = load i64, ptr %231, align 8, !noalias !33909
  %234 = getelementptr inbounds nuw i8, ptr %231, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %100, ptr noundef nonnull align 8 dereferenceable(32) %234, i64 32, i1 false), !noalias !33909
  %235 = icmp eq i64 %233, 0
  br i1 %235, label %.loopexit294, label %236

236:                                              ; preds = %227
  call void @llvm.lifetime.start.p0(ptr nonnull %99)
  store i64 %233, ptr %99, align 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %212, ptr noundef nonnull align 8 dereferenceable(32) %100, i64 32, i1 false)
  call void @llvm.lifetime.start.p0(ptr nonnull %98)
; invoke <purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::pass::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::pass::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %98, ptr noalias nofree noundef align 8 dereferenceable(200) %123, ptr noundef nonnull align 16 %4)
          to label %318 unwind label %540

.loopexit294:                                     ; preds = %420, %227, %199
  %237 = phi ptr [ %203, %199 ], [ %206, %420 ], [ %232, %227 ]
  store ptr %237, ptr %207, align 8
  br label %238

238:                                              ; preds = %531, %.loopexit294
  %239 = phi ptr [ %232, %531 ], [ %237, %.loopexit294 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %100)
  %240 = ptrtoint ptr %206 to i64
  %241 = ptrtoint ptr %239 to i64
  %242 = sub nuw i64 %240, %241
  %243 = udiv exact i64 %242, 40
  call void @llvm.experimental.noalias.scope.decl(metadata !33912)
  %244 = icmp eq ptr %206, %239
  br i1 %244, label %.loopexit289, label %.preheader288

.preheader288:                                    ; preds = %238
  %245 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %246 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  br label %247

247:                                              ; preds = %.preheader288, %285
  %248 = phi i64 [ %250, %285 ], [ 0, %.preheader288 ]
  %249 = getelementptr inbounds nuw [40 x i8], ptr %239, i64 %248
  %250 = add nuw nsw i64 %248, 1
  %251 = load i64, ptr %249, align 8, !range !1940, !alias.scope !33915, !noalias !33918, !noundef !1708
  %252 = icmp ugt i64 %251, 5
  br i1 %252, label %253, label %285

253:                                              ; preds = %247
  %254 = getelementptr i8, ptr %249, i64 8
  %255 = load ptr, ptr %254, align 8, !alias.scope !33912, !noalias !33918, !nonnull !1708, !noundef !1708
  %256 = shl i64 %251, 3
  %257 = add i64 %256, -8
  %258 = load i64, ptr %245, align 8, !noalias !33923, !noundef !1708
  %259 = call i64 @llvm.umin.i64(i64 %257, i64 9223372036854775807)
  %260 = call i64 @llvm.ssub.sat.i64(i64 %258, i64 %259)
  store i64 %260, ptr %245, align 8, !noalias !33923
  %261 = load i64, ptr %246, align 8, !noalias !33923, !noundef !1708
  %262 = icmp slt i64 %260, %261
  br i1 %262, label %263, label %.preheader2216

263:                                              ; preds = %253
  store i64 %260, ptr %246, align 8, !noalias !33923
  br label %.preheader2216

.preheader2216:                                   ; preds = %263, %253
  br label %264

264:                                              ; preds = %.preheader2216, %267
  %265 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952) acquire, align 8, !noalias !33923
  %266 = icmp slt i64 %265, 0
  br i1 %266, label %267, label %__rustc::__rust_dealloc (.exit)

267:                                              ; preds = %264
  %268 = add nsw i64 %265, 1
  %269 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 %265, i64 %268 acq_rel acquire, align 8, !noalias !33923
  %270 = extractvalue { i64, i1 } %269, 1
  br i1 %270, label %271, label %264

271:                                              ; preds = %267
  %272 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %259 monotonic, align 8, !noalias !33923
  %273 = call i64 @llvm.ssub.sat.i64(i64 %272, i64 %259)
  %274 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !33923
  br label %275

275:                                              ; preds = %278, %271
  %276 = phi i64 [ %274, %271 ], [ %281, %278 ]
  %277 = icmp slt i64 %273, %276
  br i1 %277, label %278, label %282

278:                                              ; preds = %275
  %279 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %276, i64 %273 monotonic monotonic, align 8, !noalias !33923
  %280 = extractvalue { i64, i1 } %279, 1
  %281 = extractvalue { i64, i1 } %279, 0
  br i1 %280, label %282, label %275

282:                                              ; preds = %278, %275
  %283 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 1 release, align 8, !noalias !33923
  br label %__rustc::__rust_dealloc (.exit)

__rustc::__rust_dealloc (.exit): ; preds = %264, %282
  %284 = icmp ne i64 %257, 0
  call void @llvm.assume(i1 %284), !noalias !33923
  call void @free(ptr noundef nonnull %255) #88, !noalias !33923
  br label %285

285:                                              ; preds = %__rustc::__rust_dealloc (.exit), %247
  %286 = icmp eq i64 %250, %243
  br i1 %286, label %.loopexit289, label %247

.loopexit289:                                     ; preds = %285, %238
  %287 = icmp eq i64 %204, 0
  br i1 %287, label %532, label %288

288:                                              ; preds = %.loopexit289
  %289 = mul nuw i64 %204, 40
  %290 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %291 = load i64, ptr %290, align 8, !noalias !33918, !noundef !1708
  %292 = call i64 @llvm.umin.i64(i64 %289, i64 9223372036854775807)
  %293 = call i64 @llvm.ssub.sat.i64(i64 %291, i64 %292)
  store i64 %293, ptr %290, align 8, !noalias !33918
  %294 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %295 = load i64, ptr %294, align 8, !noalias !33918, !noundef !1708
  %296 = icmp slt i64 %293, %295
  br i1 %296, label %297, label %.preheader2215

297:                                              ; preds = %288
  store i64 %293, ptr %294, align 8, !noalias !33918
  br label %.preheader2215

.preheader2215:                                   ; preds = %297, %288
  br label %298

298:                                              ; preds = %.preheader2215, %301
  %299 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952) acquire, align 8, !noalias !33918
  %300 = icmp slt i64 %299, 0
  br i1 %300, label %301, label %__rustc::__rust_dealloc (.exit209)

301:                                              ; preds = %298
  %302 = add nsw i64 %299, 1
  %303 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 %299, i64 %302 acq_rel acquire, align 8, !noalias !33918
  %304 = extractvalue { i64, i1 } %303, 1
  br i1 %304, label %305, label %298

305:                                              ; preds = %301
  %306 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %292 monotonic, align 8, !noalias !33918
  %307 = call i64 @llvm.ssub.sat.i64(i64 %306, i64 %292)
  %308 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !33918
  br label %309

309:                                              ; preds = %312, %305
  %310 = phi i64 [ %308, %305 ], [ %315, %312 ]
  %311 = icmp slt i64 %307, %310
  br i1 %311, label %312, label %316

312:                                              ; preds = %309
  %313 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %310, i64 %307 monotonic monotonic, align 8, !noalias !33918
  %314 = extractvalue { i64, i1 } %313, 1
  %315 = extractvalue { i64, i1 } %313, 0
  br i1 %314, label %316, label %309

316:                                              ; preds = %312, %309
  %317 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 1 release, align 8, !noalias !33918
  br label %__rustc::__rust_dealloc (.exit209)

__rustc::__rust_dealloc (.exit209): ; preds = %298, %316
  call void @free(ptr noundef nonnull %203) #88, !noalias !33918
  br label %532

318:                                              ; preds = %236
  %319 = load i8, ptr %98, align 8, !range !1906, !noundef !1708
  %320 = icmp eq i8 %319, -1
  br i1 %320, label %321, label %355

321:                                              ; preds = %318
  call void @llvm.lifetime.end.p0(ptr nonnull %98)
  call void @llvm.lifetime.start.p0(ptr nonnull %97)
  %322 = add i64 %233, -1
  %323 = icmp ugt i64 %322, 4
  %324 = load ptr, ptr %212, align 8
  %325 = load i64, ptr %213, align 8
  %326 = add i64 %325, -1
  %327 = select i1 %323, i64 %326, i64 %322
  %328 = select i1 %323, ptr %324, ptr %212
  %329 = load ptr, ptr %124, align 8, !nonnull !1708, !noundef !1708
  %330 = getelementptr inbounds nuw i8, ptr %329, i64 16
  call void @llvm.lifetime.start.p0(ptr nonnull %9), !noalias !33926
; invoke <purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::term::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::term::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef align 16 captures(none) dereferenceable(96) %9, ptr noalias nofree noundef nonnull align 8 dereferenceable(216) %122, ptr noalias nofree noundef nonnull readonly align 4 captures(address, read_provenance) %328, i64 noundef range(i64 0, 1152921504606846976) %327, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(56) %330, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %4)
          to label %331 unwind label %540, !inline_history !33933

331:                                              ; preds = %321
  %332 = load i64, ptr %9, align 16, !range !2530, !noalias !33926, !noundef !1708
  %333 = icmp eq i64 %332, -1
  %334 = load i32, ptr %215, align 8, !noalias !33926
  %335 = load i32, ptr %216, align 4, !noalias !33926
  br i1 %333, label %341, label %336

336:                                              ; preds = %331
  %337 = getelementptr inbounds nuw i8, ptr %9, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(80) %97, ptr noundef nonnull align 16 dereferenceable(80) %337, i64 80, i1 false), !noalias !33934
  call void @llvm.lifetime.end.p0(ptr nonnull %9), !noalias !33926
  %338 = trunc i32 %334 to i8
  %339 = lshr i32 %334, 8
  %340 = trunc nuw i32 %339 to i24
  br label %361

341:                                              ; preds = %331
  call void @llvm.lifetime.end.p0(ptr nonnull %9), !noalias !33926
  %342 = icmp eq i32 %334, 2
  br i1 %342, label %343, label %344

343:                                              ; preds = %341
  call void @llvm.lifetime.end.p0(ptr nonnull %97)
  br label %378

344:                                              ; preds = %341
  call void @llvm.lifetime.start.p0(ptr nonnull %8), !noalias !33926
; invoke purrdf_sparql_eval::expr::ebv_term::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @purrdf_sparql_eval::expr::ebv_term::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef align 16 captures(none) dereferenceable(96) %8, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %4, i32 noundef %334, i32 noundef %335)
          to label %345 unwind label %540, !inline_history !33933

345:                                              ; preds = %344
  %346 = load i64, ptr %8, align 16, !range !2530, !noalias !33926, !noundef !1708
  %347 = icmp eq i64 %346, -1
  %348 = load i8, ptr %217, align 8, !noalias !33926
  br i1 %347, label %375, label %349

349:                                              ; preds = %345
  %350 = getelementptr inbounds nuw i8, ptr %8, i64 9
  %351 = load i24, ptr %350, align 1, !noalias !33934
  %352 = getelementptr inbounds nuw i8, ptr %8, i64 12
  %353 = load i32, ptr %352, align 4, !noalias !33934
  %354 = getelementptr inbounds nuw i8, ptr %8, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(80) %97, ptr noundef nonnull align 16 dereferenceable(80) %354, i64 80, i1 false), !noalias !33934
  call void @llvm.lifetime.end.p0(ptr nonnull %8), !noalias !33926
  br label %361

355:                                              ; preds = %318
  store ptr %232, ptr %207, align 8
  call void @llvm.lifetime.end.p0(ptr nonnull %98)
  %356 = icmp ugt i64 %233, 5
  br i1 %356, label %357, label %531

357:                                              ; preds = %355
  %358 = load ptr, ptr %212, align 8, !nonnull !1708, !noundef !1708
  %359 = shl i64 %233, 3
  %360 = add i64 %359, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %358, i64 noundef %360, i64 noundef range(i64 1, -9223372036854775807) 4) #88, !noalias !33935
  br label %531

361:                                              ; preds = %349, %336
  %362 = phi i24 [ %340, %336 ], [ %351, %349 ]
  %363 = phi i8 [ %338, %336 ], [ %348, %349 ]
  %364 = phi i32 [ %335, %336 ], [ %353, %349 ]
  %365 = phi i64 [ %332, %336 ], [ %346, %349 ]
  %366 = getelementptr inbounds nuw i8, ptr %0, i64 25
  store i24 %362, ptr %366, align 1
  %367 = getelementptr inbounds nuw i8, ptr %0, i64 28
  store i32 %364, ptr %367, align 4
  %368 = getelementptr inbounds nuw i8, ptr %0, i64 32
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(80) %368, ptr noundef nonnull align 16 dereferenceable(80) %97, i64 80, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %97)
  %369 = getelementptr inbounds nuw i8, ptr %0, i64 16
  store i64 %365, ptr %369, align 16
  %370 = getelementptr inbounds nuw i8, ptr %0, i64 24
  store i8 %363, ptr %370, align 8
  store i64 1, ptr %0, align 16
  %371 = icmp ugt i64 %233, 5
  br i1 %371, label %372, label %431

372:                                              ; preds = %361
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %324) ]
  %373 = shl i64 %233, 3
  %374 = add i64 %373, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %324, i64 noundef %374, i64 noundef range(i64 1, -9223372036854775807) 4) #88, !noalias !33938
  br label %431

375:                                              ; preds = %345
  call void @llvm.lifetime.end.p0(ptr nonnull %8), !noalias !33926
  call void @llvm.lifetime.end.p0(ptr nonnull %97)
  %376 = and i8 %348, 1
  %377 = icmp eq i8 %376, 0
  br i1 %377, label %378, label %410

378:                                              ; preds = %375, %343
  %379 = icmp ugt i64 %233, 5
  br i1 %379, label %380, label %420

380:                                              ; preds = %378
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %324) ]
  %381 = shl i64 %233, 3
  %382 = add i64 %381, -8
  %383 = load i64, ptr %218, align 8, !noalias !33941, !noundef !1708
  %384 = call i64 @llvm.umin.i64(i64 %382, i64 9223372036854775807)
  %385 = call i64 @llvm.ssub.sat.i64(i64 %383, i64 %384)
  store i64 %385, ptr %218, align 8, !noalias !33941
  %386 = load i64, ptr %219, align 8, !noalias !33941, !noundef !1708
  %387 = icmp slt i64 %385, %386
  br i1 %387, label %388, label %.preheader2219

388:                                              ; preds = %380
  store i64 %385, ptr %219, align 8, !noalias !33941
  br label %.preheader2219

.preheader2219:                                   ; preds = %388, %380
  br label %389

389:                                              ; preds = %.preheader2219, %392
  %390 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952) acquire, align 8, !noalias !33941
  %391 = icmp slt i64 %390, 0
  br i1 %391, label %392, label %__rustc::__rust_dealloc (.exit210)

392:                                              ; preds = %389
  %393 = add nsw i64 %390, 1
  %394 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 %390, i64 %393 acq_rel acquire, align 8, !noalias !33941
  %395 = extractvalue { i64, i1 } %394, 1
  br i1 %395, label %396, label %389

396:                                              ; preds = %392
  %397 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %384 monotonic, align 8, !noalias !33941
  %398 = call i64 @llvm.ssub.sat.i64(i64 %397, i64 %384)
  %399 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !33941
  br label %400

400:                                              ; preds = %403, %396
  %401 = phi i64 [ %399, %396 ], [ %406, %403 ]
  %402 = icmp slt i64 %398, %401
  br i1 %402, label %403, label %407

403:                                              ; preds = %400
  %404 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %401, i64 %398 monotonic monotonic, align 8, !noalias !33941
  %405 = extractvalue { i64, i1 } %404, 1
  %406 = extractvalue { i64, i1 } %404, 0
  br i1 %405, label %407, label %400

407:                                              ; preds = %403, %400
  %408 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 1 release, align 8, !noalias !33941
  br label %__rustc::__rust_dealloc (.exit210)

__rustc::__rust_dealloc (.exit210): ; preds = %389, %407
  %409 = icmp ne i64 %382, 0
  call void @llvm.assume(i1 %409), !noalias !33941
  call void @free(ptr noundef nonnull %324) #88, !noalias !33941
  br label %420

410:                                              ; preds = %375
  call void @llvm.lifetime.start.p0(ptr nonnull %96)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %96, ptr noundef nonnull align 8 dereferenceable(24) %214, i64 24, i1 false)
  call void @llvm.experimental.noalias.scope.decl(metadata !33944)
  %411 = load i64, ptr %102, align 8, !range !1817, !alias.scope !33944, !noalias !33947, !noundef !1708
  %412 = icmp eq i64 %229, %411
  br i1 %412, label %413, label %425

413:                                              ; preds = %410
; invoke <alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one
  invoke void @<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %102)
          to label %414 unwind label %416, !noalias !33947

414:                                              ; preds = %413
  %415 = load ptr, ptr %200, align 8, !alias.scope !33944, !noalias !33947
  br label %425

416:                                              ; preds = %413
  %417 = landingpad { ptr, i32 }
          cleanup
  store ptr %232, ptr %207, align 8
  %418 = icmp ugt i64 %233, 5
  br i1 %418, label %419, label %225

419:                                              ; preds = %416
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %324) ]
  br label %220

420:                                              ; preds = %425, %__rustc::__rust_dealloc (.exit210), %378
  %421 = phi ptr [ %228, %__rustc::__rust_dealloc (.exit210) ], [ %228, %378 ], [ %426, %425 ]
  %422 = phi i64 [ %229, %__rustc::__rust_dealloc (.exit210) ], [ %229, %378 ], [ %430, %425 ]
  %423 = phi ptr [ %230, %__rustc::__rust_dealloc (.exit210) ], [ %230, %378 ], [ %426, %425 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %99)
  call void @llvm.lifetime.end.p0(ptr nonnull %100)
  call void @llvm.lifetime.start.p0(ptr nonnull %100)
  %424 = icmp eq ptr %232, %206
  br i1 %424, label %.loopexit294, label %227

425:                                              ; preds = %414, %410
  %426 = phi ptr [ %415, %414 ], [ %228, %410 ]
  %427 = getelementptr inbounds nuw [40 x i8], ptr %426, i64 %229
  store i64 %233, ptr %427, align 8, !noalias !33944
  %428 = getelementptr inbounds nuw i8, ptr %427, i64 8
  store ptr %324, ptr %428, align 8, !noalias !33944
  %429 = getelementptr inbounds nuw i8, ptr %427, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %429, ptr noundef nonnull align 8 dereferenceable(24) %96, i64 24, i1 false), !noalias !33944
  %430 = add i64 %229, 1
  store i64 %430, ptr %201, align 8, !alias.scope !33944, !noalias !33947
  call void @llvm.lifetime.end.p0(ptr nonnull %96)
  br label %420

431:                                              ; preds = %372, %361
  call void @llvm.lifetime.end.p0(ptr nonnull %99)
  call void @llvm.lifetime.end.p0(ptr nonnull %100)
  %432 = ptrtoint ptr %206 to i64
  %433 = ptrtoint ptr %232 to i64
  %434 = sub nuw i64 %432, %433
  %435 = udiv exact i64 %434, 40
  call void @llvm.experimental.noalias.scope.decl(metadata !33949)
  %436 = icmp eq ptr %206, %232
  br i1 %436, label %.loopexit293, label %.preheader292

.preheader292:                                    ; preds = %431, %474
  %437 = phi i64 [ %439, %474 ], [ 0, %431 ]
  %438 = getelementptr inbounds nuw [40 x i8], ptr %232, i64 %437
  %439 = add nuw nsw i64 %437, 1
  %440 = load i64, ptr %438, align 8, !range !1940, !alias.scope !33952, !noalias !33955, !noundef !1708
  %441 = icmp ugt i64 %440, 5
  br i1 %441, label %442, label %474

442:                                              ; preds = %.preheader292
  %443 = getelementptr i8, ptr %438, i64 8
  %444 = load ptr, ptr %443, align 8, !alias.scope !33949, !noalias !33955, !nonnull !1708, !noundef !1708
  %445 = shl i64 %440, 3
  %446 = add i64 %445, -8
  %447 = load i64, ptr %218, align 8, !noalias !33960, !noundef !1708
  %448 = call i64 @llvm.umin.i64(i64 %446, i64 9223372036854775807)
  %449 = call i64 @llvm.ssub.sat.i64(i64 %447, i64 %448)
  store i64 %449, ptr %218, align 8, !noalias !33960
  %450 = load i64, ptr %219, align 8, !noalias !33960, !noundef !1708
  %451 = icmp slt i64 %449, %450
  br i1 %451, label %452, label %.preheader2218

452:                                              ; preds = %442
  store i64 %449, ptr %219, align 8, !noalias !33960
  br label %.preheader2218

.preheader2218:                                   ; preds = %452, %442
  br label %453

453:                                              ; preds = %.preheader2218, %456
  %454 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952) acquire, align 8, !noalias !33960
  %455 = icmp slt i64 %454, 0
  br i1 %455, label %456, label %__rustc::__rust_dealloc (.exit211)

456:                                              ; preds = %453
  %457 = add nsw i64 %454, 1
  %458 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 %454, i64 %457 acq_rel acquire, align 8, !noalias !33960
  %459 = extractvalue { i64, i1 } %458, 1
  br i1 %459, label %460, label %453

460:                                              ; preds = %456
  %461 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %448 monotonic, align 8, !noalias !33960
  %462 = call i64 @llvm.ssub.sat.i64(i64 %461, i64 %448)
  %463 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !33960
  br label %464

464:                                              ; preds = %467, %460
  %465 = phi i64 [ %463, %460 ], [ %470, %467 ]
  %466 = icmp slt i64 %462, %465
  br i1 %466, label %467, label %471

467:                                              ; preds = %464
  %468 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %465, i64 %462 monotonic monotonic, align 8, !noalias !33960
  %469 = extractvalue { i64, i1 } %468, 1
  %470 = extractvalue { i64, i1 } %468, 0
  br i1 %469, label %471, label %464

471:                                              ; preds = %467, %464
  %472 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 1 release, align 8, !noalias !33960
  br label %__rustc::__rust_dealloc (.exit211)

__rustc::__rust_dealloc (.exit211): ; preds = %453, %471
  %473 = icmp ne i64 %446, 0
  call void @llvm.assume(i1 %473), !noalias !33960
  call void @free(ptr noundef nonnull %444) #88, !noalias !33960
  br label %474

474:                                              ; preds = %__rustc::__rust_dealloc (.exit211), %.preheader292
  %475 = icmp eq i64 %439, %435
  br i1 %475, label %.loopexit293, label %.preheader292

.loopexit293:                                     ; preds = %474, %431
  %476 = icmp eq i64 %204, 0
  br i1 %476, label %479, label %477

477:                                              ; preds = %.loopexit293
  %478 = mul nuw i64 %204, 40
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %203, i64 noundef %478, i64 noundef range(i64 1, -9223372036854775807) 8) #88, !noalias !33955
  br label %479

479:                                              ; preds = %477, %.loopexit293
  call void @llvm.lifetime.end.p0(ptr nonnull %101)
  call void @llvm.experimental.noalias.scope.decl(metadata !33963)
  call void @llvm.experimental.noalias.scope.decl(metadata !33966)
  %480 = icmp eq i64 %229, 0
  br i1 %480, label %.loopexit291, label %.preheader290

.preheader290:                                    ; preds = %479, %518
  %481 = phi i64 [ %483, %518 ], [ 0, %479 ]
  %482 = getelementptr inbounds nuw [40 x i8], ptr %230, i64 %481
  %483 = add nuw nsw i64 %481, 1
  %484 = load i64, ptr %482, align 8, !range !1940, !alias.scope !33969, !noalias !33963, !noundef !1708
  %485 = icmp ugt i64 %484, 5
  br i1 %485, label %486, label %518

486:                                              ; preds = %.preheader290
  %487 = getelementptr i8, ptr %482, i64 8
  %488 = load ptr, ptr %487, align 8, !alias.scope !33966, !noalias !33963, !nonnull !1708, !noundef !1708
  %489 = shl i64 %484, 3
  %490 = add i64 %489, -8
  %491 = load i64, ptr %218, align 8, !noalias !33972, !noundef !1708
  %492 = call i64 @llvm.umin.i64(i64 %490, i64 9223372036854775807)
  %493 = call i64 @llvm.ssub.sat.i64(i64 %491, i64 %492)
  store i64 %493, ptr %218, align 8, !noalias !33972
  %494 = load i64, ptr %219, align 8, !noalias !33972, !noundef !1708
  %495 = icmp slt i64 %493, %494
  br i1 %495, label %496, label %.preheader2217

496:                                              ; preds = %486
  store i64 %493, ptr %219, align 8, !noalias !33972
  br label %.preheader2217

.preheader2217:                                   ; preds = %496, %486
  br label %497

497:                                              ; preds = %.preheader2217, %500
  %498 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952) acquire, align 8, !noalias !33972
  %499 = icmp slt i64 %498, 0
  br i1 %499, label %500, label %__rustc::__rust_dealloc (.exit212)

500:                                              ; preds = %497
  %501 = add nsw i64 %498, 1
  %502 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 %498, i64 %501 acq_rel acquire, align 8, !noalias !33972
  %503 = extractvalue { i64, i1 } %502, 1
  br i1 %503, label %504, label %497

504:                                              ; preds = %500
  %505 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %492 monotonic, align 8, !noalias !33972
  %506 = call i64 @llvm.ssub.sat.i64(i64 %505, i64 %492)
  %507 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !33972
  br label %508

508:                                              ; preds = %511, %504
  %509 = phi i64 [ %507, %504 ], [ %514, %511 ]
  %510 = icmp slt i64 %506, %509
  br i1 %510, label %511, label %515

511:                                              ; preds = %508
  %512 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %509, i64 %506 monotonic monotonic, align 8, !noalias !33972
  %513 = extractvalue { i64, i1 } %512, 1
  %514 = extractvalue { i64, i1 } %512, 0
  br i1 %513, label %515, label %508

515:                                              ; preds = %511, %508
  %516 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 1 release, align 8, !noalias !33972
  br label %__rustc::__rust_dealloc (.exit212)

__rustc::__rust_dealloc (.exit212): ; preds = %497, %515
  %517 = icmp ne i64 %490, 0
  call void @llvm.assume(i1 %517), !noalias !33972
  call void @free(ptr noundef nonnull %488) #88, !noalias !33972
  br label %518

518:                                              ; preds = %__rustc::__rust_dealloc (.exit212), %.preheader290
  %519 = icmp eq i64 %483, %229
  br i1 %519, label %.loopexit291, label %.preheader290

.loopexit291:                                     ; preds = %518, %479
  %520 = load i64, ptr %102, align 8, !alias.scope !33963
  %521 = icmp eq i64 %520, 0
  br i1 %521, label %530, label %522

522:                                              ; preds = %.loopexit291
  %523 = mul nuw i64 %520, 40
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %230, i64 noundef %523, i64 noundef range(i64 1, -9223372036854775807) 8) #88, !noalias !33963
  br label %530

524:                                              ; preds = %2979, %2732, %573, %570, %566, %528, %225
  %525 = phi i8 [ 1, %528 ], [ 0, %225 ], [ %534, %2979 ], [ %534, %2732 ], [ 1, %573 ], [ 1, %566 ], [ 1, %570 ]
  %526 = phi i1 [ true, %528 ], [ true, %225 ], [ true, %2979 ], [ false, %2732 ], [ true, %573 ], [ true, %566 ], [ true, %570 ]
  %527 = phi { ptr, i32 } [ %529, %528 ], [ %226, %225 ], [ %2980, %2979 ], [ %2733, %2732 ], [ %567, %573 ], [ %567, %566 ], [ %567, %570 ]
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>(ptr noalias nofree noundef align 8 dereferenceable(216) %122) #89
          to label %187 unwind label %545

528:                                              ; preds = %2987, %2443, %547
  %529 = landingpad { ptr, i32 }
          cleanup
  br label %524

530:                                              ; preds = %522, %.loopexit291
  call void @llvm.lifetime.end.p0(ptr nonnull %102)
  br label %2882

531:                                              ; preds = %357, %355
  call void @llvm.lifetime.end.p0(ptr nonnull %99)
  br label %238

532:                                              ; preds = %__rustc::__rust_dealloc (.exit209), %.loopexit289
  call void @llvm.lifetime.end.p0(ptr nonnull %101)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %121, ptr noundef nonnull align 8 dereferenceable(24) %102, i64 24, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %102)
  br label %533

533:                                              ; preds = %2731, %532
  %534 = phi i8 [ 1, %2731 ], [ 0, %532 ]
  %535 = getelementptr inbounds nuw i8, ptr %4, i64 696
  call void @llvm.experimental.noalias.scope.decl(metadata !33975)
  %536 = load ptr, ptr %535, align 8, !alias.scope !33975, !noalias !33978, !nonnull !1708, !noundef !1708
  %537 = getelementptr inbounds nuw i8, ptr %536, i64 40
  %538 = load atomic i32, ptr %537 acquire, align 4, !noalias !33980
  %539 = icmp eq i32 %538, 0
  br i1 %539, label %2734, label %2744

540:                                              ; preds = %344, %321, %236
  %541 = landingpad { ptr, i32 }
          cleanup
  store ptr %232, ptr %207, align 8
  %542 = icmp ugt i64 %233, 5
  br i1 %542, label %543, label %225

543:                                              ; preds = %540
  %544 = load ptr, ptr %212, align 8, !nonnull !1708, !noundef !1708
  br label %220

545:                                              ; preds = %3193, %3187, %3006, %2988, %2508, %573, %524, %187
  %546 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #86
  unreachable

547:                                              ; preds = %198
  %548 = getelementptr inbounds nuw i8, ptr %123, i64 184
  %549 = load i64, ptr %548, align 8, !noundef !1708
  %550 = tail call noundef range(i64 0, 230584300921369396) i64 @llvm.umin.i64(i64 %549, i64 range(i64 0, 230584300921369396) %183)
  %551 = getelementptr inbounds nuw i8, ptr %127, i64 8
  %552 = load ptr, ptr %551, align 8, !nonnull !1708, !noundef !1708
  call void @llvm.lifetime.start.p0(ptr nonnull %120)
; invoke <purrdf_sparql_eval::eval::EvalCtx>::loop_snapshot
  %553 = invoke fastcc noundef ptr @<purrdf_sparql_eval::eval::EvalCtx>::loop_snapshot(ptr noundef nonnull align 16 %4, i64 noundef %550)
          to label %554 unwind label %528

554:                                              ; preds = %547
  store ptr %553, ptr %120, align 8
  call void @llvm.lifetime.start.p0(ptr nonnull %118)
  call void @llvm.lifetime.start.p0(ptr nonnull %117)
  %555 = getelementptr inbounds nuw i8, ptr %4, i64 616
  %556 = load ptr, ptr %555, align 8, !noundef !1708
  %557 = icmp eq ptr %556, null
  br i1 %557, label %576, label %558

558:                                              ; preds = %554
  %559 = getelementptr inbounds nuw i8, ptr %556, i64 16
  %560 = load i64, ptr %559, align 8
  %561 = icmp ugt i64 %560, -3
  br i1 %561, label %562, label %576

562:                                              ; preds = %558
  %563 = getelementptr inbounds nuw i8, ptr %556, i64 40
  %564 = load i64, ptr %563, align 8
  %565 = icmp ult i64 %564, -2
  br label %576

566:                                              ; preds = %2990, %2988, %2429, %988, %783, %779, %674, %574
  %567 = phi { ptr, i32 } [ %2991, %2990 ], [ %989, %988 ], [ %675, %674 ], [ %575, %574 ], [ %780, %783 ], [ %780, %779 ], [ %2989, %2988 ], [ %2430, %2429 ]
  call void @llvm.experimental.noalias.scope.decl(metadata !33981)
  %568 = load ptr, ptr %120, align 8, !alias.scope !33981, !noundef !1708
  %569 = icmp eq ptr %568, null
  br i1 %569, label %524, label %570

570:                                              ; preds = %566
  %571 = atomicrmw sub ptr %568, i64 1 release, align 8, !noalias !33984
  %572 = icmp eq i64 %571, 1
  br i1 %572, label %573, label %524

573:                                              ; preds = %570
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %120) #87
          to label %524 unwind label %545, !inline_history !1744

574:                                              ; preds = %697, %600, %699, %685, %678, %593
  %575 = landingpad { ptr, i32 }
          cleanup
  br label %566

576:                                              ; preds = %562, %558, %554
  %577 = phi i1 [ false, %554 ], [ true, %558 ], [ %565, %562 ]
  %578 = getelementptr inbounds nuw i8, ptr %4, i64 1234
  %579 = load i8, ptr %578, align 2, !range !1746, !noundef !1708
  %580 = trunc nuw i8 %579 to i1
  call void @llvm.lifetime.start.p0(ptr nonnull %116)
  store ptr %4, ptr %116, align 8
  %581 = getelementptr inbounds nuw i8, ptr %116, i64 8
  store ptr %120, ptr %581, align 8
  %582 = getelementptr inbounds nuw i8, ptr %116, i64 16
  store ptr %123, ptr %582, align 8
  %583 = getelementptr inbounds nuw i8, ptr %116, i64 24
  store ptr %122, ptr %583, align 8
  call void @llvm.lifetime.start.p0(ptr nonnull %115)
  store ptr %552, ptr %115, align 8
  %584 = getelementptr inbounds nuw i8, ptr %115, i64 8
  store i64 %550, ptr %584, align 8
  %585 = getelementptr inbounds nuw i8, ptr %115, i64 16
  store ptr %124, ptr %585, align 8
  br i1 %577, label %682, label %586

586:                                              ; preds = %576
  call void @llvm.lifetime.start.p0(ptr nonnull %20)
  call void @llvm.lifetime.start.p0(ptr nonnull %21)
  store ptr %552, ptr %21, align 8, !noalias !33987
  %587 = getelementptr inbounds nuw i8, ptr %21, i64 8
  store i64 %550, ptr %587, align 8, !noalias !33987
  store ptr %123, ptr %20, align 8, !noalias !33987
  call void @llvm.lifetime.start.p0(ptr nonnull %19), !noalias !33987
  store ptr %116, ptr %19, align 8, !noalias !33987
  %588 = getelementptr inbounds nuw i8, ptr %19, i64 8
  store ptr %21, ptr %588, align 8, !noalias !33987
  %589 = getelementptr inbounds nuw i8, ptr %19, i64 16
  store ptr %115, ptr %589, align 8, !noalias !33987
  %590 = getelementptr inbounds nuw i8, ptr %19, i64 24
  store ptr %20, ptr %590, align 8, !noalias !33987
  %591 = icmp samesign ult i64 %550, 1025
  %592 = or i1 %591, %580
  br i1 %592, label %593, label %594

593:                                              ; preds = %586
; invoke purrdf_sparql_eval::parallel::par_chunk_try_map_init::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_filter<purrdf_core::ir::dataset::RdfDataset>::{closure#0}, purrdf_sparql_eval::expr::eval_filter<purrdf_core::ir::dataset::RdfDataset>::{closure#1}, purrdf_sparql_eval::expr::eval_filter<purrdf_core::ir::dataset::RdfDataset>::{closure#2}>::{closure#0}
  invoke fastcc void @purrdf_sparql_eval::parallel::par_chunk_try_map_init::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_filter<purrdf_core::ir::dataset::RdfDataset>::{closure#0}, purrdf_sparql_eval::expr::eval_filter<purrdf_core::ir::dataset::RdfDataset>::{closure#1}, purrdf_sparql_eval::expr::eval_filter<purrdf_core::ir::dataset::RdfDataset>::{closure#2}>::{closure#0}(ptr noalias nofree noundef nonnull align 16 captures(none) dereferenceable(256) %117, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(32) %19) #91
          to label %681 unwind label %574, !inline_history !33993

594:                                              ; preds = %586
  %595 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @rayon_core::registry::WORKER_THREAD_STATE::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL)
  %596 = load ptr, ptr %595, align 8, !noundef !1708
  %597 = icmp eq ptr %596, null
  br i1 %597, label %600, label %598

598:                                              ; preds = %594
  %599 = getelementptr inbounds nuw i8, ptr %596, i64 272
  br label %602

600:                                              ; preds = %594
; invoke rayon_core::registry::global_registry
  %601 = invoke noundef nonnull align 8 ptr @rayon_core::registry::global_registry()
          to label %._crit_edge unwind label %574

._crit_edge:                                      ; preds = %600
  %.pre = load ptr, ptr %21, align 8, !noalias !33987
  %.pre875 = load i64, ptr %587, align 8, !noalias !33987
  br label %602

602:                                              ; preds = %._crit_edge, %598
  %603 = phi i64 [ %550, %598 ], [ %.pre875, %._crit_edge ]
  %604 = phi ptr [ %552, %598 ], [ %.pre, %._crit_edge ]
  %605 = phi ptr [ %599, %598 ], [ %601, %._crit_edge ]
  %606 = load ptr, ptr %605, align 8, !nonnull !1708, !noundef !1708
  %607 = getelementptr inbounds nuw i8, ptr %606, i64 520
  %608 = load i64, ptr %607, align 8, !noundef !1708
  %609 = icmp ult i64 %608, 192153584101141163
  call void @llvm.assume(i1 %609)
  %610 = call i64 @llvm.umax.i64(i64 %608, i64 1)
  %611 = shl nuw nsw i64 %610, 2
  %612 = udiv i64 %550, %611
  %613 = call noundef range(i64 16, 0) i64 @llvm.umax.i64(i64 %612, i64 16)
  call void @llvm.lifetime.start.p0(ptr nonnull %18), !noalias !33987
  call void @llvm.lifetime.start.p0(ptr nonnull %17), !noalias !33994
  store i64 0, ptr %17, align 8, !alias.scope !33998, !noalias !33994
  %614 = getelementptr inbounds nuw i8, ptr %17, i64 8
  store ptr inttoptr (i64 16 to ptr), ptr %614, align 8, !alias.scope !33998, !noalias !33994
  %615 = getelementptr inbounds nuw i8, ptr %17, i64 16
  store i64 0, ptr %615, align 8, !alias.scope !33998, !noalias !33994
  call void @llvm.experimental.noalias.scope.decl(metadata !34001)
  %616 = udiv i64 %603, %613
  %617 = urem i64 %603, %613
  %618 = icmp ne i64 %617, 0
  %619 = zext i1 %618 to i64
  %620 = add nuw nsw i64 %616, %619
  call void @llvm.experimental.noalias.scope.decl(metadata !34004), !noalias !34007
  call void @llvm.lifetime.start.p0(ptr nonnull %16), !noalias !34008
  store i64 %620, ptr %16, align 8, !noalias !34010
  %621 = icmp eq i64 %620, 0
  br i1 %621, label %626, label %622, !prof !1974

622:                                              ; preds = %602
; invoke <alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global>
  invoke fastcc void @<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.13412714042204560522)(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %17, i64 noundef 0, i64 noundef %620, i64 noundef 16, i64 noundef 256)
          to label %623 unwind label %672, !inline_history !34012

623:                                              ; preds = %622
  %624 = load i64, ptr %615, align 8, !alias.scope !34013, !noalias !34016
  %625 = load i64, ptr %17, align 8, !range !1817, !alias.scope !34013, !noalias !34016
  br label %626

626:                                              ; preds = %623, %602
  %627 = phi i64 [ %625, %623 ], [ 0, %602 ]
  %628 = phi i64 [ %624, %623 ], [ 0, %602 ]
  call void @llvm.lifetime.start.p0(ptr nonnull %15), !noalias !34010
  %629 = icmp ult i64 %628, 36028797018963968
  call void @llvm.assume(i1 %629), !noalias !34017
  %630 = sub nsw i64 %627, %628
  %631 = icmp ult i64 %630, %620
  br i1 %631, label %632, label %634, !prof !1803

632:                                              ; preds = %626
; invoke core::panicking::panic
  invoke void @core::panicking::panic(ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @anon.a12f493ba210922c94e5446ac885c35e.965, i64 noundef 47, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.a12f493ba210922c94e5446ac885c35e.967) #92
          to label %633 unwind label %672, !inline_history !34012

633:                                              ; preds = %632
  unreachable

634:                                              ; preds = %626
  %635 = load ptr, ptr %614, align 8, !alias.scope !34013, !noalias !34016, !nonnull !1708, !noundef !1708
  call void @llvm.lifetime.start.p0(ptr nonnull %12), !noalias !34018
  store ptr %604, ptr %12, align 8, !noalias !34022
  %636 = getelementptr inbounds nuw i8, ptr %12, i64 8
  store i64 %603, ptr %636, align 8, !noalias !34022
  %637 = getelementptr inbounds nuw i8, ptr %12, i64 16
  store i64 %613, ptr %637, align 8, !noalias !34022
  %638 = getelementptr inbounds nuw i8, ptr %12, i64 24
  store ptr %116, ptr %638, align 8, !noalias !34022
  %639 = getelementptr inbounds nuw i8, ptr %12, i64 32
  store ptr %115, ptr %639, align 8, !noalias !34022
  %640 = getelementptr inbounds nuw i8, ptr %12, i64 40
  store ptr %20, ptr %640, align 8, !noalias !34022
  call void @llvm.lifetime.start.p0(ptr nonnull %11), !noalias !34023
  %641 = getelementptr inbounds nuw i8, ptr %11, i64 16
  store i64 %613, ptr %641, align 8, !noalias !34023
  store ptr %604, ptr %11, align 8, !noalias !34023
  %642 = getelementptr inbounds nuw i8, ptr %11, i64 8
  store i64 %603, ptr %642, align 8, !noalias !34023
  %643 = load ptr, ptr %595, align 8, !noundef !1708
  %644 = icmp eq ptr %643, null
  br i1 %644, label %647, label %645

645:                                              ; preds = %634
  %646 = getelementptr inbounds nuw i8, ptr %643, i64 272
  br label %649

647:                                              ; preds = %634
; invoke rayon_core::registry::global_registry
  %648 = invoke noundef nonnull align 8 ptr @rayon_core::registry::global_registry()
          to label %649 unwind label %672

649:                                              ; preds = %645, %647
  %650 = phi ptr [ %646, %645 ], [ %648, %647 ]
  %651 = load ptr, ptr %650, align 8, !nonnull !1708, !noundef !1708
  %652 = getelementptr inbounds nuw i8, ptr %651, i64 520
  %653 = load i64, ptr %652, align 8, !noundef !1708
  %654 = icmp ult i64 %653, 192153584101141163
  call void @llvm.assume(i1 %654)
  %655 = getelementptr inbounds nuw [256 x i8], ptr %635, i64 %628
  call void @llvm.lifetime.start.p0(ptr nonnull %10), !noalias !34034
  store ptr %638, ptr %10, align 8, !noalias !34039
  %656 = getelementptr inbounds nuw i8, ptr %10, i64 8
  store ptr %655, ptr %656, align 8, !noalias !34039
  %657 = getelementptr inbounds nuw i8, ptr %10, i64 16
  store i64 %620, ptr %657, align 8, !noalias !34039
; invoke rayon::iter::plumbing::bridge_producer_consumer::helper::<rayon::slice::chunks::ChunksProducer<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>, rayon::iter::map::MapConsumer<rayon::iter::collect::consumer::CollectConsumer<core::result::Result<(alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)), purrdf_sparql_eval::error::EvalError>>, purrdf_sparql_eval::parallel::par_chunk_try_map_init<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_filter<purrdf_core::ir::dataset::RdfDataset>::{closure#0}, purrdf_sparql_eval::expr::eval_filter<purrdf_core::ir::dataset::RdfDataset>::{closure#1}, purrdf_sparql_eval::expr::eval_filter<purrdf_core::ir::dataset::RdfDataset>::{closure#2}>::{closure#1}>>
  invoke fastcc void @rayon::iter::plumbing::bridge_producer_consumer::helper::<rayon::slice::chunks::ChunksProducer<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>, rayon::iter::map::MapConsumer<rayon::iter::collect::consumer::CollectConsumer<core::result::Result<(alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)), purrdf_sparql_eval::error::EvalError>>, purrdf_sparql_eval::parallel::par_chunk_try_map_init<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_filter<purrdf_core::ir::dataset::RdfDataset>::{closure#0}, purrdf_sparql_eval::expr::eval_filter<purrdf_core::ir::dataset::RdfDataset>::{closure#1}, purrdf_sparql_eval::expr::eval_filter<purrdf_core::ir::dataset::RdfDataset>::{closure#2}>::{closure#1}>>(ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(24) %15, i64 noundef %620, i1 noundef zeroext false, i64 noundef %653, i64 noundef 1, ptr noalias nofree noundef nonnull readonly align 8 captures(none) dereferenceable(24) %11, ptr noalias nofree noundef readonly align 8 captures(address) dereferenceable(24) %10)
          to label %658 unwind label %672, !inline_history !34040

658:                                              ; preds = %649
  call void @llvm.lifetime.end.p0(ptr nonnull %10), !noalias !34034
  call void @llvm.lifetime.end.p0(ptr nonnull %11), !noalias !34023
  call void @llvm.lifetime.end.p0(ptr nonnull %12), !noalias !34018
  call void @llvm.lifetime.start.p0(ptr nonnull %14), !noalias !34010
  %659 = getelementptr inbounds nuw i8, ptr %15, i64 16
  %660 = load i64, ptr %659, align 8, !noalias !34010, !noundef !1708
  store i64 %660, ptr %14, align 8, !noalias !34010
  %661 = icmp eq i64 %660, %620
  br i1 %661, label %678, label %662, !prof !1974

662:                                              ; preds = %658
  call void @llvm.lifetime.start.p0(ptr nonnull %13), !noalias !34010
  store ptr %16, ptr %13, align 8, !noalias !34010
  %663 = getelementptr inbounds nuw i8, ptr %13, i64 8
  store ptr @<usize as core::fmt::Display>::fmt, ptr %663, align 8, !noalias !34010
  %664 = getelementptr inbounds nuw i8, ptr %13, i64 16
  store ptr %14, ptr %664, align 8, !noalias !34010
  %665 = getelementptr inbounds nuw i8, ptr %13, i64 24
  store ptr @<usize as core::fmt::Display>::fmt, ptr %665, align 8, !noalias !34010
; invoke core::panicking::panic_fmt
  invoke void @core::panicking::panic_fmt(ptr noundef nonnull @anon.a12f493ba210922c94e5446ac885c35e.550, ptr noundef nonnull %13, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.a12f493ba210922c94e5446ac885c35e.552) #90
          to label %669 unwind label %666, !noalias !34010, !inline_history !34041

666:                                              ; preds = %662
  %667 = landingpad { ptr, i32 }
          cleanup
  %668 = load ptr, ptr %15, align 8, !noalias !34010, !noundef !1708
; invoke core::ptr::drop_glue::<rayon::iter::collect::consumer::CollectResult<core::result::Result<(alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)), purrdf_sparql_eval::error::EvalError>>>
  invoke fastcc void @core::ptr::drop_glue::<rayon::iter::collect::consumer::CollectResult<core::result::Result<(alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)), purrdf_sparql_eval::error::EvalError>>>(ptr %668, i64 %660) #89
          to label %674 unwind label %670, !noalias !34010, !inline_history !34041

669:                                              ; preds = %662
  unreachable

670:                                              ; preds = %666
  %671 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #86, !noalias !34010, !inline_history !34041
  unreachable

672:                                              ; preds = %647, %649, %632, %622
  %673 = landingpad { ptr, i32 }
          cleanup
  br label %674

674:                                              ; preds = %672, %666
  %675 = phi { ptr, i32 } [ %673, %672 ], [ %667, %666 ]
; invoke core::ptr::drop_glue::<alloc::vec::Vec<core::result::Result<(alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)), purrdf_sparql_eval::error::EvalError>>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<core::result::Result<(alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)), purrdf_sparql_eval::error::EvalError>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %17) #89
          to label %566 unwind label %676, !noalias !34007, !inline_history !34042

676:                                              ; preds = %674
  %677 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #86, !noalias !34007, !inline_history !34042
  unreachable

678:                                              ; preds = %658
  %679 = add nuw nsw i64 %628, %620
  store i64 %679, ptr %615, align 8, !alias.scope !34043, !noalias !34016
  call void @llvm.lifetime.end.p0(ptr nonnull %14), !noalias !34010
  call void @llvm.lifetime.end.p0(ptr nonnull %15), !noalias !34010
  call void @llvm.lifetime.end.p0(ptr nonnull %16), !noalias !34008
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %18, ptr noundef nonnull align 8 dereferenceable(24) %17, i64 24, i1 false), !noalias !34044
  call void @llvm.lifetime.end.p0(ptr nonnull %17), !noalias !33994
; invoke purrdf_sparql_eval::parallel::concatenate_chunks::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)>
  invoke fastcc void @purrdf_sparql_eval::parallel::concatenate_chunks::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)>(ptr noalias nofree noundef nonnull align 16 captures(none) dereferenceable(256) %117, ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %18)
          to label %680 unwind label %574, !inline_history !33993

680:                                              ; preds = %678
  call void @llvm.lifetime.end.p0(ptr nonnull %18), !noalias !33987
  br label %681

681:                                              ; preds = %680, %593
  call void @llvm.lifetime.end.p0(ptr nonnull %19), !noalias !33987
  call void @llvm.lifetime.end.p0(ptr nonnull %20)
  call void @llvm.lifetime.end.p0(ptr nonnull %21)
  br label %957

682:                                              ; preds = %576
  call void @llvm.lifetime.start.p0(ptr nonnull %36)
  store ptr %123, ptr %36, align 8, !noalias !34045
  %683 = icmp samesign ult i64 %550, 1025
  %684 = or i1 %683, %580
  br i1 %684, label %685, label %691

685:                                              ; preds = %682
  call void @llvm.lifetime.start.p0(ptr nonnull %35), !noalias !34045
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %35, ptr noundef nonnull readonly align 8 dereferenceable(32) %116, i64 32, i1 false), !noalias !34051
  call void @llvm.lifetime.start.p0(ptr nonnull %34), !noalias !34045
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %34, ptr noundef nonnull readonly align 8 dereferenceable(24) %115, i64 24, i1 false), !noalias !34052
  call void @llvm.lifetime.start.p0(ptr nonnull %25), !noalias !34045
  call void @llvm.lifetime.start.p0(ptr nonnull %26), !noalias !34045
  store ptr %552, ptr %26, align 8, !noalias !34053
  %686 = getelementptr inbounds nuw i8, ptr %26, i64 8
  store i64 %550, ptr %686, align 8, !noalias !34053
  store ptr %123, ptr %25, align 8, !noalias !34053
  call void @llvm.lifetime.start.p0(ptr nonnull %24), !noalias !34053
  store ptr %35, ptr %24, align 8, !noalias !34053
  %687 = getelementptr inbounds nuw i8, ptr %24, i64 8
  store ptr %26, ptr %687, align 8, !noalias !34053
  %688 = getelementptr inbounds nuw i8, ptr %24, i64 16
  store ptr %34, ptr %688, align 8, !noalias !34053
  %689 = getelementptr inbounds nuw i8, ptr %24, i64 24
  store ptr %25, ptr %689, align 8, !noalias !34053
; invoke purrdf_sparql_eval::parallel::par_chunk_try_map_init::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_filter<purrdf_core::ir::dataset::RdfDataset>::{closure#0}, purrdf_sparql_eval::expr::eval_filter<purrdf_core::ir::dataset::RdfDataset>::{closure#1}, purrdf_sparql_eval::expr::eval_filter<purrdf_core::ir::dataset::RdfDataset>::{closure#2}>::{closure#0}
  invoke fastcc void @purrdf_sparql_eval::parallel::par_chunk_try_map_init::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_filter<purrdf_core::ir::dataset::RdfDataset>::{closure#0}, purrdf_sparql_eval::expr::eval_filter<purrdf_core::ir::dataset::RdfDataset>::{closure#1}, purrdf_sparql_eval::expr::eval_filter<purrdf_core::ir::dataset::RdfDataset>::{closure#2}>::{closure#0}(ptr noalias nofree noundef nonnull align 16 captures(none) dereferenceable(256) %117, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(32) %24) #91
          to label %690 unwind label %574, !inline_history !34059

690:                                              ; preds = %685
  call void @llvm.lifetime.end.p0(ptr nonnull %24), !noalias !34053
  call void @llvm.lifetime.end.p0(ptr nonnull %25), !noalias !34045
  call void @llvm.lifetime.end.p0(ptr nonnull %26), !noalias !34045
  call void @llvm.lifetime.end.p0(ptr nonnull %34), !noalias !34045
  call void @llvm.lifetime.end.p0(ptr nonnull %35), !noalias !34045
  br label %956

691:                                              ; preds = %682
  %692 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @rayon_core::registry::WORKER_THREAD_STATE::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL)
  %693 = load ptr, ptr %692, align 8, !noundef !1708
  %694 = icmp eq ptr %693, null
  br i1 %694, label %697, label %695

695:                                              ; preds = %691
  %696 = getelementptr inbounds nuw i8, ptr %693, i64 272
  br label %699

697:                                              ; preds = %691
; invoke rayon_core::registry::global_registry
  %698 = invoke noundef nonnull align 8 ptr @rayon_core::registry::global_registry()
          to label %699 unwind label %574

699:                                              ; preds = %695, %697
  %700 = phi ptr [ %696, %695 ], [ %698, %697 ]
  %701 = load ptr, ptr %700, align 8, !nonnull !1708, !noundef !1708
  %702 = getelementptr inbounds nuw i8, ptr %701, i64 520
  %703 = load i64, ptr %702, align 8, !noundef !1708
  %704 = icmp ult i64 %703, 192153584101141163
  call void @llvm.assume(i1 %704)
  %705 = call i64 @llvm.umax.i64(i64 %703, i64 1)
  %706 = shl nuw i64 %705, 6
  call void @llvm.lifetime.start.p0(ptr nonnull %33), !noalias !34045
  call void @llvm.lifetime.start.p0(ptr nonnull %32), !noalias !34045
  %707 = udiv i64 %550, %706
  %708 = call i64 @llvm.umax.i64(i64 %707, i64 64)
  store ptr %552, ptr %32, align 8, !noalias !34045
  %709 = getelementptr inbounds nuw i8, ptr %32, i64 8
  store i64 %550, ptr %709, align 8, !noalias !34045
  %710 = getelementptr inbounds nuw i8, ptr %32, i64 16
  store i64 %708, ptr %710, align 8, !noalias !34045
; invoke <alloc::vec::Vec<&[purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>]> as alloc::vec::spec_from_iter::SpecFromIter<&[purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>], core::slice::iter::Chunks<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>>::from_iter
  invoke fastcc void @<alloc::vec::Vec<&[purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>]> as alloc::vec::spec_from_iter::SpecFromIter<&[purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>], core::slice::iter::Chunks<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>>::from_iter(ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %33, ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %32)
          to label %711 unwind label %574, !inline_history !34059

711:                                              ; preds = %699
  call void @llvm.lifetime.end.p0(ptr nonnull %32), !noalias !34045
  call void @llvm.lifetime.start.p0(ptr nonnull %31), !noalias !34045
  %712 = getelementptr inbounds nuw i8, ptr %33, i64 8
  %713 = getelementptr inbounds nuw i8, ptr %33, i64 16
  %714 = load i64, ptr %713, align 8, !noalias !34045, !noundef !1708
  call void @llvm.experimental.noalias.scope.decl(metadata !34060)
  call void @llvm.experimental.noalias.scope.decl(metadata !34063)
  %715 = mul i64 %714, 272
  %716 = icmp ugt i64 %714, 33909456017848440
  br i1 %716, label %722, label %717, !prof !5895

717:                                              ; preds = %711
  %718 = icmp eq i64 %715, 0
  br i1 %718, label %725, label %719

719:                                              ; preds = %717
; call __rustc::__rust_alloc
  %720 = call noundef align 16 ptr @__rustc::__rust_alloc(i64 noundef %715, i64 noundef range(i64 1, 17) 16) #88, !noalias !34066, !inline_history !34059
  %721 = icmp eq ptr %720, null
  br i1 %721, label %722, label %725

722:                                              ; preds = %719, %711
  %723 = phi i64 [ 16, %719 ], [ 0, %711 ]
; invoke alloc::raw_vec::handle_error
  invoke void @alloc::raw_vec::handle_error(i64 noundef %723, i64 %715) #90
          to label %724 unwind label %786, !noalias !34069, !inline_history !34059

724:                                              ; preds = %722
  unreachable

725:                                              ; preds = %719, %717
  %726 = phi i64 [ 0, %717 ], [ %714, %719 ]
  %727 = phi ptr [ inttoptr (i64 16 to ptr), %717 ], [ %720, %719 ]
  %728 = icmp samesign ule i64 %714, %726
  call void @llvm.assume(i1 %728)
  %729 = icmp eq i64 %714, 0
  br i1 %729, label %.loopexit287, label %.preheader286.preheader

.preheader286.preheader:                          ; preds = %725
  %xtraiter = and i64 %714, 7
  %730 = icmp ult i64 %714, 8
  br i1 %730, label %.preheader286.epil.preheader, label %.preheader286.preheader.new

.preheader286.preheader.new:                      ; preds = %.preheader286.preheader
  %unroll_iter = and i64 %714, 36028797018963960
  br label %.preheader286

.preheader286:                                    ; preds = %.preheader286, %.preheader286.preheader.new
  %731 = phi i64 [ 0, %.preheader286.preheader.new ], [ %763, %.preheader286 ]
  %niter = phi i64 [ 0, %.preheader286.preheader.new ], [ %niter.next.7, %.preheader286 ]
  %732 = getelementptr inbounds nuw [272 x i8], ptr %727, i64 %731
  store i32 0, ptr %732, align 16, !noalias !34070
  %733 = getelementptr inbounds nuw i8, ptr %732, i64 4
  store i8 0, ptr %733, align 4, !noalias !34070
  %734 = getelementptr inbounds nuw i8, ptr %732, i64 16
  store i64 2, ptr %734, align 16, !noalias !34070
  %735 = getelementptr inbounds nuw [272 x i8], ptr %727, i64 %731
  %736 = getelementptr inbounds nuw i8, ptr %735, i64 272
  store i32 0, ptr %736, align 16, !noalias !34070
  %737 = getelementptr inbounds nuw i8, ptr %735, i64 276
  store i8 0, ptr %737, align 4, !noalias !34070
  %738 = getelementptr inbounds nuw i8, ptr %735, i64 288
  store i64 2, ptr %738, align 16, !noalias !34070
  %739 = getelementptr inbounds nuw [272 x i8], ptr %727, i64 %731
  %740 = getelementptr inbounds nuw i8, ptr %739, i64 544
  store i32 0, ptr %740, align 16, !noalias !34070
  %741 = getelementptr inbounds nuw i8, ptr %739, i64 548
  store i8 0, ptr %741, align 4, !noalias !34070
  %742 = getelementptr inbounds nuw i8, ptr %739, i64 560
  store i64 2, ptr %742, align 16, !noalias !34070
  %743 = getelementptr inbounds nuw [272 x i8], ptr %727, i64 %731
  %744 = getelementptr inbounds nuw i8, ptr %743, i64 816
  store i32 0, ptr %744, align 16, !noalias !34070
  %745 = getelementptr inbounds nuw i8, ptr %743, i64 820
  store i8 0, ptr %745, align 4, !noalias !34070
  %746 = getelementptr inbounds nuw i8, ptr %743, i64 832
  store i64 2, ptr %746, align 16, !noalias !34070
  %747 = getelementptr inbounds nuw [272 x i8], ptr %727, i64 %731
  %748 = getelementptr inbounds nuw i8, ptr %747, i64 1088
  store i32 0, ptr %748, align 16, !noalias !34070
  %749 = getelementptr inbounds nuw i8, ptr %747, i64 1092
  store i8 0, ptr %749, align 4, !noalias !34070
  %750 = getelementptr inbounds nuw i8, ptr %747, i64 1104
  store i64 2, ptr %750, align 16, !noalias !34070
  %751 = getelementptr inbounds nuw [272 x i8], ptr %727, i64 %731
  %752 = getelementptr inbounds nuw i8, ptr %751, i64 1360
  store i32 0, ptr %752, align 16, !noalias !34070
  %753 = getelementptr inbounds nuw i8, ptr %751, i64 1364
  store i8 0, ptr %753, align 4, !noalias !34070
  %754 = getelementptr inbounds nuw i8, ptr %751, i64 1376
  store i64 2, ptr %754, align 16, !noalias !34070
  %755 = getelementptr inbounds nuw [272 x i8], ptr %727, i64 %731
  %756 = getelementptr inbounds nuw i8, ptr %755, i64 1632
  store i32 0, ptr %756, align 16, !noalias !34070
  %757 = getelementptr inbounds nuw i8, ptr %755, i64 1636
  store i8 0, ptr %757, align 4, !noalias !34070
  %758 = getelementptr inbounds nuw i8, ptr %755, i64 1648
  store i64 2, ptr %758, align 16, !noalias !34070
  %759 = getelementptr inbounds nuw [272 x i8], ptr %727, i64 %731
  %760 = getelementptr inbounds nuw i8, ptr %759, i64 1904
  store i32 0, ptr %760, align 16, !noalias !34070
  %761 = getelementptr inbounds nuw i8, ptr %759, i64 1908
  store i8 0, ptr %761, align 4, !noalias !34070
  %762 = getelementptr inbounds nuw i8, ptr %759, i64 1920
  store i64 2, ptr %762, align 16, !noalias !34070
  %763 = add nuw i64 %731, 8
  %niter.next.7 = add i64 %niter, 8
  %niter.ncmp.7 = icmp eq i64 %niter.next.7, %unroll_iter
  br i1 %niter.ncmp.7, label %.loopexit287.loopexit.unr-lcssa, label %.preheader286

.loopexit287.loopexit.unr-lcssa:                  ; preds = %.preheader286
  %lcmp.mod.not = icmp eq i64 %xtraiter, 0
  br i1 %lcmp.mod.not, label %.loopexit287, label %.preheader286.epil.preheader

.preheader286.epil.preheader:                     ; preds = %.loopexit287.loopexit.unr-lcssa, %.preheader286.preheader
  %.epil.init = phi i64 [ 0, %.preheader286.preheader ], [ %763, %.loopexit287.loopexit.unr-lcssa ]
  %lcmp.mod2261 = icmp ne i64 %xtraiter, 0
  call void @llvm.assume(i1 %lcmp.mod2261)
  br label %.preheader286.epil

.preheader286.epil:                               ; preds = %.preheader286.epil, %.preheader286.epil.preheader
  %764 = phi i64 [ %768, %.preheader286.epil ], [ %.epil.init, %.preheader286.epil.preheader ]
  %epil.iter = phi i64 [ %epil.iter.next, %.preheader286.epil ], [ 0, %.preheader286.epil.preheader ]
  %765 = getelementptr inbounds nuw [272 x i8], ptr %727, i64 %764
  store i32 0, ptr %765, align 16, !noalias !34070
  %766 = getelementptr inbounds nuw i8, ptr %765, i64 4
  store i8 0, ptr %766, align 4, !noalias !34070
  %767 = getelementptr inbounds nuw i8, ptr %765, i64 16
  store i64 2, ptr %767, align 16, !noalias !34070
  %768 = add nuw i64 %764, 1
  %epil.iter.next = add i64 %epil.iter, 1
  %epil.iter.cmp.not = icmp eq i64 %epil.iter.next, %xtraiter
  br i1 %epil.iter.cmp.not, label %.loopexit287, label %.preheader286.epil, !llvm.loop !34087

.loopexit287:                                     ; preds = %.loopexit287.loopexit.unr-lcssa, %.preheader286.epil, %725
  store i64 %726, ptr %31, align 8, !alias.scope !34088, !noalias !34045
  %769 = getelementptr inbounds nuw i8, ptr %31, i64 8
  store ptr %727, ptr %769, align 8, !alias.scope !34088, !noalias !34045
  %770 = getelementptr inbounds nuw i8, ptr %31, i64 16
  store i64 %714, ptr %770, align 8, !alias.scope !34088, !noalias !34045
  call void @llvm.lifetime.start.p0(ptr nonnull %30), !noalias !34045
  store i64 0, ptr %30, align 8, !noalias !34045
  %771 = load i64, ptr %713, align 8, !noalias !34045, !noundef !1708
  %772 = icmp ult i64 %771, 576460752303423488
  call void @llvm.assume(i1 %772)
  %773 = call i64 @llvm.umin.i64(i64 %705, i64 %771)
  call void @llvm.lifetime.start.p0(ptr nonnull %29), !noalias !34045
  store ptr %30, ptr %29, align 8, !noalias !34045
  %774 = getelementptr inbounds nuw i8, ptr %29, i64 8
  store ptr %33, ptr %774, align 8, !noalias !34045
  %775 = getelementptr inbounds nuw i8, ptr %29, i64 16
  store ptr %116, ptr %775, align 8, !noalias !34045
  %776 = getelementptr inbounds nuw i8, ptr %29, i64 24
  store ptr %115, ptr %776, align 8, !noalias !34045
  %777 = getelementptr inbounds nuw i8, ptr %29, i64 32
  store ptr %36, ptr %777, align 8, !noalias !34045
  %778 = getelementptr inbounds nuw i8, ptr %29, i64 40
  store ptr %31, ptr %778, align 8, !noalias !34045
; invoke <rayon::range::Iter<usize> as rayon::iter::ParallelIterator>::drive_unindexed::<rayon::iter::for_each::ForEachConsumer<purrdf_sparql_eval::parallel::par_blocks_try_map_init<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_filter<purrdf_core::ir::dataset::RdfDataset>::{closure#0}, purrdf_sparql_eval::expr::eval_filter<purrdf_core::ir::dataset::RdfDataset>::{closure#1}, purrdf_sparql_eval::expr::eval_filter<purrdf_core::ir::dataset::RdfDataset>::{closure#2}>::{closure#1}>>
  invoke fastcc void @<rayon::range::Iter<usize> as rayon::iter::ParallelIterator>::drive_unindexed::<rayon::iter::for_each::ForEachConsumer<purrdf_sparql_eval::parallel::par_blocks_try_map_init<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_filter<purrdf_core::ir::dataset::RdfDataset>::{closure#0}, purrdf_sparql_eval::expr::eval_filter<purrdf_core::ir::dataset::RdfDataset>::{closure#1}, purrdf_sparql_eval::expr::eval_filter<purrdf_core::ir::dataset::RdfDataset>::{closure#2}>::{closure#1}>>(i64 noundef range(i64 0, 576460752303423488) %773, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(48) %29)
          to label %788 unwind label %952

779:                                              ; preds = %952, %818, %816, %786
  %780 = phi { ptr, i32 } [ %953, %952 ], [ %817, %816 ], [ %787, %786 ], [ %926, %818 ]
  %781 = load i64, ptr %33, align 8, !noalias !34045
  %782 = icmp eq i64 %781, 0
  br i1 %782, label %566, label %783

783:                                              ; preds = %779
  %784 = load ptr, ptr %712, align 8, !noalias !34045, !nonnull !1708, !noundef !1708
  %785 = shl nuw i64 %781, 4
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %784, i64 noundef %785, i64 noundef range(i64 1, -9223372036854775807) 8) #88, !noalias !34069, !inline_history !34059
  br label %566

786:                                              ; preds = %722
  %787 = landingpad { ptr, i32 }
          cleanup
  br label %779

788:                                              ; preds = %.loopexit287
  call void @llvm.lifetime.end.p0(ptr nonnull %29), !noalias !34045
  call void @llvm.lifetime.start.p0(ptr nonnull %28), !noalias !34045
  call void @llvm.lifetime.start.p0(ptr nonnull %27), !noalias !34045
  %789 = load ptr, ptr %769, align 8, !noalias !34045, !nonnull !1708, !noundef !1708
  %790 = load i64, ptr %31, align 8, !range !1817, !noalias !34045, !noundef !1708
  %791 = load i64, ptr %770, align 8, !noalias !34045, !noundef !1708
  %792 = icmp ult i64 %791, 33909456017848441
  call void @llvm.assume(i1 %792)
  %793 = mul nuw i64 %791, 272
  %794 = getelementptr inbounds nuw i8, ptr %789, i64 %793
  %795 = getelementptr inbounds nuw i8, ptr %27, i64 8
  %796 = getelementptr inbounds nuw i8, ptr %27, i64 16
  %797 = getelementptr inbounds nuw i8, ptr %27, i64 24
  call void @llvm.experimental.noalias.scope.decl(metadata !34089)
  call void @llvm.experimental.noalias.scope.decl(metadata !34092)
  call void @llvm.experimental.noalias.scope.decl(metadata !34094)
  call void @llvm.experimental.noalias.scope.decl(metadata !34097)
  %798 = mul i64 %790, 272
  %799 = icmp eq i64 %791, 0
  br i1 %799, label %.loopexit285, label %.preheader284.preheader

.preheader284.preheader:                          ; preds = %788
  %800 = add i64 %793, -272
  %801 = udiv i64 %800, 272
  %802 = add nuw nsw i64 %801, 1
  %xtraiter2262 = and i64 %802, 7
  %lcmp.mod2263.not = icmp eq i64 %xtraiter2262, 0
  br i1 %lcmp.mod2263.not, label %.preheader284.prol.loopexit, label %.preheader284.prol

.preheader284.prol:                               ; preds = %.preheader284.preheader, %813
  %803 = phi ptr [ %808, %813 ], [ %789, %.preheader284.preheader ]
  %804 = phi ptr [ %814, %813 ], [ %789, %.preheader284.preheader ]
  %prol.iter = phi i64 [ %prol.iter.next, %813 ], [ 0, %.preheader284.preheader ]
  %805 = getelementptr inbounds nuw i8, ptr %803, i64 16
  %806 = load i64, ptr %805, align 16, !noalias !34099
  %807 = getelementptr inbounds nuw i8, ptr %803, i64 24
  call void @llvm.lifetime.start.p0(ptr nonnull %22)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(248) %22, ptr noundef nonnull align 8 dereferenceable(248) %807, i64 248, i1 false), !noalias !34099
  %808 = getelementptr inbounds nuw i8, ptr %803, i64 272
  %809 = icmp eq i64 %806, 2
  br i1 %809, label %813, label %810

810:                                              ; preds = %.preheader284.prol
  store i64 %806, ptr %804, align 16, !noalias !34106
  %811 = getelementptr inbounds nuw i8, ptr %804, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(248) %811, ptr noundef nonnull align 8 dereferenceable(248) %22, i64 248, i1 false), !noalias !34099
  %812 = getelementptr inbounds nuw i8, ptr %804, i64 256
  br label %813

813:                                              ; preds = %810, %.preheader284.prol
  %814 = phi ptr [ %812, %810 ], [ %804, %.preheader284.prol ]
  call void @llvm.lifetime.end.p0(ptr nonnull %22)
  %prol.iter.next = add i64 %prol.iter, 1
  %prol.iter.cmp.not = icmp eq i64 %prol.iter.next, %xtraiter2262
  br i1 %prol.iter.cmp.not, label %.preheader284.prol.loopexit, label %.preheader284.prol, !llvm.loop !34109

.preheader284.prol.loopexit:                      ; preds = %813, %.preheader284.preheader
  %.lcssa2214.unr = phi ptr [ poison, %.preheader284.preheader ], [ %814, %813 ]
  %.unr2264 = phi ptr [ %789, %.preheader284.preheader ], [ %808, %813 ]
  %.unr2265 = phi ptr [ %789, %.preheader284.preheader ], [ %814, %813 ]
  %815 = icmp ult i64 %800, 1904
  br i1 %815, label %.loopexit285, label %.preheader284

816:                                              ; preds = %944, %939
  %817 = landingpad { ptr, i32 }
          cleanup
  br label %779

818:                                              ; preds = %.loopexit281
; invoke core::ptr::drop_glue::<core::iter::adapters::filter_map::FilterMap<alloc::vec::into_iter::IntoIter<std::sync::poison::mutex::Mutex<core::option::Option<core::result::Result<(alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)), purrdf_sparql_eval::error::EvalError>>>>, purrdf_sparql_eval::parallel::par_blocks_try_map_init<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_filter<purrdf_core::ir::dataset::RdfDataset>::{closure#0}, purrdf_sparql_eval::expr::eval_filter<purrdf_core::ir::dataset::RdfDataset>::{closure#1}, purrdf_sparql_eval::expr::eval_filter<purrdf_core::ir::dataset::RdfDataset>::{closure#2}>::{closure#2}>>
  invoke fastcc void @core::ptr::drop_glue::<core::iter::adapters::filter_map::FilterMap<alloc::vec::into_iter::IntoIter<std::sync::poison::mutex::Mutex<core::option::Option<core::result::Result<(alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)), purrdf_sparql_eval::error::EvalError>>>>, purrdf_sparql_eval::parallel::par_blocks_try_map_init<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_filter<purrdf_core::ir::dataset::RdfDataset>::{closure#0}, purrdf_sparql_eval::expr::eval_filter<purrdf_core::ir::dataset::RdfDataset>::{closure#1}, purrdf_sparql_eval::expr::eval_filter<purrdf_core::ir::dataset::RdfDataset>::{closure#2}>::{closure#2}>>(ptr noalias nofree noundef nonnull align 8 dereferenceable(32) %27) #89
          to label %779 unwind label %937, !noalias !34110, !inline_history !34059

.preheader284:                                    ; preds = %.preheader284.prol.loopexit, %885
  %819 = phi ptr [ %880, %885 ], [ %.unr2264, %.preheader284.prol.loopexit ]
  %820 = phi ptr [ %886, %885 ], [ %.unr2265, %.preheader284.prol.loopexit ]
  %821 = getelementptr inbounds nuw i8, ptr %819, i64 16
  %822 = load i64, ptr %821, align 16, !noalias !34099
  %823 = getelementptr inbounds nuw i8, ptr %819, i64 24
  call void @llvm.lifetime.start.p0(ptr nonnull %22)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(248) %22, ptr noundef nonnull align 8 dereferenceable(248) %823, i64 248, i1 false), !noalias !34099
  %824 = icmp eq i64 %822, 2
  br i1 %824, label %.preheader284.1, label %825

825:                                              ; preds = %.preheader284
  store i64 %822, ptr %820, align 16, !noalias !34106
  %826 = getelementptr inbounds nuw i8, ptr %820, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(248) %826, ptr noundef nonnull align 8 dereferenceable(248) %22, i64 248, i1 false), !noalias !34099
  %827 = getelementptr inbounds nuw i8, ptr %820, i64 256
  br label %.preheader284.1

.preheader284.1:                                  ; preds = %825, %.preheader284
  %828 = phi ptr [ %827, %825 ], [ %820, %.preheader284 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %22)
  %829 = getelementptr inbounds nuw i8, ptr %819, i64 288
  %830 = load i64, ptr %829, align 16, !noalias !34099
  %831 = getelementptr inbounds nuw i8, ptr %819, i64 296
  call void @llvm.lifetime.start.p0(ptr nonnull %22)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(248) %22, ptr noundef nonnull align 8 dereferenceable(248) %831, i64 248, i1 false), !noalias !34099
  %832 = icmp eq i64 %830, 2
  br i1 %832, label %.preheader284.2, label %833

833:                                              ; preds = %.preheader284.1
  store i64 %830, ptr %828, align 16, !noalias !34106
  %834 = getelementptr inbounds nuw i8, ptr %828, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(248) %834, ptr noundef nonnull align 8 dereferenceable(248) %22, i64 248, i1 false), !noalias !34099
  %835 = getelementptr inbounds nuw i8, ptr %828, i64 256
  br label %.preheader284.2

.preheader284.2:                                  ; preds = %833, %.preheader284.1
  %836 = phi ptr [ %835, %833 ], [ %828, %.preheader284.1 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %22)
  %837 = getelementptr inbounds nuw i8, ptr %819, i64 560
  %838 = load i64, ptr %837, align 16, !noalias !34099
  %839 = getelementptr inbounds nuw i8, ptr %819, i64 568
  call void @llvm.lifetime.start.p0(ptr nonnull %22)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(248) %22, ptr noundef nonnull align 8 dereferenceable(248) %839, i64 248, i1 false), !noalias !34099
  %840 = icmp eq i64 %838, 2
  br i1 %840, label %.preheader284.3, label %841

841:                                              ; preds = %.preheader284.2
  store i64 %838, ptr %836, align 16, !noalias !34106
  %842 = getelementptr inbounds nuw i8, ptr %836, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(248) %842, ptr noundef nonnull align 8 dereferenceable(248) %22, i64 248, i1 false), !noalias !34099
  %843 = getelementptr inbounds nuw i8, ptr %836, i64 256
  br label %.preheader284.3

.preheader284.3:                                  ; preds = %841, %.preheader284.2
  %844 = phi ptr [ %843, %841 ], [ %836, %.preheader284.2 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %22)
  %845 = getelementptr inbounds nuw i8, ptr %819, i64 832
  %846 = load i64, ptr %845, align 16, !noalias !34099
  %847 = getelementptr inbounds nuw i8, ptr %819, i64 840
  call void @llvm.lifetime.start.p0(ptr nonnull %22)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(248) %22, ptr noundef nonnull align 8 dereferenceable(248) %847, i64 248, i1 false), !noalias !34099
  %848 = icmp eq i64 %846, 2
  br i1 %848, label %.preheader284.4, label %849

849:                                              ; preds = %.preheader284.3
  store i64 %846, ptr %844, align 16, !noalias !34106
  %850 = getelementptr inbounds nuw i8, ptr %844, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(248) %850, ptr noundef nonnull align 8 dereferenceable(248) %22, i64 248, i1 false), !noalias !34099
  %851 = getelementptr inbounds nuw i8, ptr %844, i64 256
  br label %.preheader284.4

.preheader284.4:                                  ; preds = %849, %.preheader284.3
  %852 = phi ptr [ %851, %849 ], [ %844, %.preheader284.3 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %22)
  %853 = getelementptr inbounds nuw i8, ptr %819, i64 1104
  %854 = load i64, ptr %853, align 16, !noalias !34099
  %855 = getelementptr inbounds nuw i8, ptr %819, i64 1112
  call void @llvm.lifetime.start.p0(ptr nonnull %22)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(248) %22, ptr noundef nonnull align 8 dereferenceable(248) %855, i64 248, i1 false), !noalias !34099
  %856 = icmp eq i64 %854, 2
  br i1 %856, label %.preheader284.5, label %857

857:                                              ; preds = %.preheader284.4
  store i64 %854, ptr %852, align 16, !noalias !34106
  %858 = getelementptr inbounds nuw i8, ptr %852, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(248) %858, ptr noundef nonnull align 8 dereferenceable(248) %22, i64 248, i1 false), !noalias !34099
  %859 = getelementptr inbounds nuw i8, ptr %852, i64 256
  br label %.preheader284.5

.preheader284.5:                                  ; preds = %857, %.preheader284.4
  %860 = phi ptr [ %859, %857 ], [ %852, %.preheader284.4 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %22)
  %861 = getelementptr inbounds nuw i8, ptr %819, i64 1376
  %862 = load i64, ptr %861, align 16, !noalias !34099
  %863 = getelementptr inbounds nuw i8, ptr %819, i64 1384
  call void @llvm.lifetime.start.p0(ptr nonnull %22)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(248) %22, ptr noundef nonnull align 8 dereferenceable(248) %863, i64 248, i1 false), !noalias !34099
  %864 = icmp eq i64 %862, 2
  br i1 %864, label %.preheader284.6, label %865

865:                                              ; preds = %.preheader284.5
  store i64 %862, ptr %860, align 16, !noalias !34106
  %866 = getelementptr inbounds nuw i8, ptr %860, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(248) %866, ptr noundef nonnull align 8 dereferenceable(248) %22, i64 248, i1 false), !noalias !34099
  %867 = getelementptr inbounds nuw i8, ptr %860, i64 256
  br label %.preheader284.6

.preheader284.6:                                  ; preds = %865, %.preheader284.5
  %868 = phi ptr [ %867, %865 ], [ %860, %.preheader284.5 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %22)
  %869 = getelementptr inbounds nuw i8, ptr %819, i64 1648
  %870 = load i64, ptr %869, align 16, !noalias !34099
  %871 = getelementptr inbounds nuw i8, ptr %819, i64 1656
  call void @llvm.lifetime.start.p0(ptr nonnull %22)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(248) %22, ptr noundef nonnull align 8 dereferenceable(248) %871, i64 248, i1 false), !noalias !34099
  %872 = icmp eq i64 %870, 2
  br i1 %872, label %.preheader284.7, label %873

873:                                              ; preds = %.preheader284.6
  store i64 %870, ptr %868, align 16, !noalias !34106
  %874 = getelementptr inbounds nuw i8, ptr %868, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(248) %874, ptr noundef nonnull align 8 dereferenceable(248) %22, i64 248, i1 false), !noalias !34099
  %875 = getelementptr inbounds nuw i8, ptr %868, i64 256
  br label %.preheader284.7

.preheader284.7:                                  ; preds = %873, %.preheader284.6
  %876 = phi ptr [ %875, %873 ], [ %868, %.preheader284.6 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %22)
  %877 = getelementptr inbounds nuw i8, ptr %819, i64 1920
  %878 = load i64, ptr %877, align 16, !noalias !34099
  %879 = getelementptr inbounds nuw i8, ptr %819, i64 1928
  call void @llvm.lifetime.start.p0(ptr nonnull %22)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(248) %22, ptr noundef nonnull align 8 dereferenceable(248) %879, i64 248, i1 false), !noalias !34099
  %880 = getelementptr inbounds nuw i8, ptr %819, i64 2176
  %881 = icmp eq i64 %878, 2
  br i1 %881, label %885, label %882

882:                                              ; preds = %.preheader284.7
  store i64 %878, ptr %876, align 16, !noalias !34106
  %883 = getelementptr inbounds nuw i8, ptr %876, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(248) %883, ptr noundef nonnull align 8 dereferenceable(248) %22, i64 248, i1 false), !noalias !34099
  %884 = getelementptr inbounds nuw i8, ptr %876, i64 256
  br label %885

885:                                              ; preds = %882, %.preheader284.7
  %886 = phi ptr [ %884, %882 ], [ %876, %.preheader284.7 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %22)
  %887 = icmp eq ptr %880, %794
  br i1 %887, label %.loopexit285, label %.preheader284

.loopexit285:                                     ; preds = %.preheader284.prol.loopexit, %885, %788
  %888 = phi ptr [ %789, %788 ], [ %794, %885 ], [ %794, %.preheader284.prol.loopexit ]
  %889 = phi ptr [ %789, %788 ], [ %.lcssa2214.unr, %.preheader284.prol.loopexit ], [ %886, %885 ]
  %890 = ptrtoint ptr %889 to i64
  %891 = ptrtoint ptr %789 to i64
  %892 = sub nuw i64 %890, %891
  %893 = lshr exact i64 %892, 8
  call void @llvm.lifetime.start.p0(ptr nonnull %23), !noalias !34111
  store ptr %789, ptr %23, align 8, !noalias !34111
  %894 = getelementptr inbounds nuw i8, ptr %23, i64 8
  store i64 %893, ptr %894, align 8, !noalias !34111
  %895 = getelementptr inbounds nuw i8, ptr %23, i64 16
  store i64 %790, ptr %895, align 8, !noalias !34111
  call void @llvm.experimental.noalias.scope.decl(metadata !34112)
  %896 = ptrtoint ptr %794 to i64
  %897 = ptrtoint ptr %888 to i64
  %898 = sub nuw i64 %896, %897
  %899 = udiv exact i64 %898, 272
  store i64 0, ptr %796, align 8, !alias.scope !34115, !noalias !34116
  store ptr inttoptr (i64 16 to ptr), ptr %27, align 8, !alias.scope !34115, !noalias !34116
  store ptr inttoptr (i64 16 to ptr), ptr %795, align 8, !alias.scope !34115, !noalias !34116
  store ptr inttoptr (i64 16 to ptr), ptr %797, align 8, !alias.scope !34115, !noalias !34116
  call void @llvm.experimental.noalias.scope.decl(metadata !34117)
  %900 = icmp eq ptr %794, %888
  br i1 %900, label %.loopexit283, label %.preheader282

.preheader282:                                    ; preds = %.loopexit285, %908
  %901 = phi i64 [ %903, %908 ], [ 0, %.loopexit285 ]
  %902 = getelementptr inbounds nuw [272 x i8], ptr %888, i64 %901
  %903 = add nuw nsw i64 %901, 1
  %904 = getelementptr inbounds nuw i8, ptr %902, i64 16
  %905 = load i64, ptr %904, align 16, !range !12503, !alias.scope !34120, !noalias !34127, !noundef !1708
  %906 = icmp eq i64 %905, 2
  br i1 %906, label %908, label %907

907:                                              ; preds = %.preheader282
; invoke core::ptr::drop_glue::<core::result::Result<(alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)), purrdf_sparql_eval::error::EvalError>>
  invoke fastcc void @core::ptr::drop_glue::<core::result::Result<(alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)), purrdf_sparql_eval::error::EvalError>>(ptr noalias nofree noundef nonnull readonly align 16 dereferenceable(256) %904)
          to label %908 unwind label %910, !noalias !34127, !inline_history !34059

908:                                              ; preds = %907, %.preheader282
  %909 = icmp eq i64 %903, %899
  br i1 %909, label %.loopexit283, label %.preheader282

910:                                              ; preds = %907
  %911 = landingpad { ptr, i32 }
          cleanup
  %912 = icmp eq i64 %903, %899
  br i1 %912, label %.loopexit281, label %.preheader280

.preheader280:                                    ; preds = %910, %920
  %913 = phi i64 [ %915, %920 ], [ %903, %910 ]
  %914 = getelementptr inbounds nuw [272 x i8], ptr %888, i64 %913
  %915 = add i64 %913, 1
  %916 = getelementptr inbounds nuw i8, ptr %914, i64 16
  %917 = load i64, ptr %916, align 16, !range !12503, !alias.scope !34128, !noalias !34127, !noundef !1708
  %918 = icmp eq i64 %917, 2
  br i1 %918, label %920, label %919

919:                                              ; preds = %.preheader280
; invoke core::ptr::drop_glue::<core::result::Result<(alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)), purrdf_sparql_eval::error::EvalError>>
  invoke fastcc void @core::ptr::drop_glue::<core::result::Result<(alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)), purrdf_sparql_eval::error::EvalError>>(ptr noalias nofree noundef nonnull readonly align 16 dereferenceable(256) %916)
          to label %920 unwind label %922, !noalias !34127, !inline_history !34059

920:                                              ; preds = %919, %.preheader280
  %921 = icmp eq i64 %915, %899
  br i1 %921, label %.loopexit281, label %.preheader280

922:                                              ; preds = %919
  %923 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #86, !noalias !34135, !inline_history !34059
  unreachable

924:                                              ; preds = %935
  %925 = landingpad { ptr, i32 }
          cleanup
  br label %.loopexit281

.loopexit281:                                     ; preds = %920, %924, %910
  %926 = phi { ptr, i32 } [ %925, %924 ], [ %911, %910 ], [ %911, %920 ]
; invoke core::ptr::drop_glue::<alloc::vec::in_place_drop::InPlaceDstDataSrcBufDrop<std::sync::poison::mutex::Mutex<core::option::Option<core::result::Result<(alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)), purrdf_sparql_eval::error::EvalError>>>, core::result::Result<(alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)), purrdf_sparql_eval::error::EvalError>>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::in_place_drop::InPlaceDstDataSrcBufDrop<std::sync::poison::mutex::Mutex<core::option::Option<core::result::Result<(alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)), purrdf_sparql_eval::error::EvalError>>>, core::result::Result<(alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)), purrdf_sparql_eval::error::EvalError>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %23) #89
          to label %818 unwind label %937, !noalias !34136, !inline_history !34059

.loopexit283:                                     ; preds = %908, %.loopexit285
  %927 = and i64 %798, 240
  %928 = icmp eq i64 %927, 0
  br i1 %928, label %939, label %929

929:                                              ; preds = %.loopexit283
  %930 = and i64 %798, -256
  %931 = icmp eq i64 %930, 0
  br i1 %931, label %932, label %933

932:                                              ; preds = %929
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %789, i64 noundef %798, i64 noundef 16) #88, !noalias !34136, !inline_history !34059
  br label %939

933:                                              ; preds = %929
; call <purrdf_alloc_probe::CountingAllocator as core::alloc::global::GlobalAlloc>::realloc
  %_0.i = call noalias noundef align 16 ptr @<purrdf_alloc_probe::CountingAllocator as core::alloc::global::GlobalAlloc>::realloc(ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @qualification_454_native_cost::GLOBAL (.llvm.11174181910260379007), ptr noundef nonnull %789, i64 noundef 16, i64 noundef %798, i64 noundef %930) #88, !noalias !34136
  %934 = icmp eq ptr %_0.i, null
  br i1 %934, label %935, label %939, !prof !4226

935:                                              ; preds = %933
; invoke alloc::alloc::handle_alloc_error
  invoke void @alloc::alloc::handle_alloc_error(i64 noundef 16, i64 noundef %930) #90
          to label %936 unwind label %924, !noalias !34136, !inline_history !34059

936:                                              ; preds = %935
  unreachable

937:                                              ; preds = %.loopexit281, %818
  %938 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #86, !noalias !34136, !inline_history !34059
  unreachable

939:                                              ; preds = %933, %932, %.loopexit283
  %940 = phi ptr [ %789, %.loopexit283 ], [ %_0.i, %933 ], [ inttoptr (i64 16 to ptr), %932 ]
  %941 = lshr i64 %798, 8
  store i64 %941, ptr %28, align 8, !alias.scope !34137, !noalias !34138
  %942 = getelementptr inbounds nuw i8, ptr %28, i64 8
  store ptr %940, ptr %942, align 8, !alias.scope !34137, !noalias !34138
  %943 = getelementptr inbounds nuw i8, ptr %28, i64 16
  store i64 %893, ptr %943, align 8, !alias.scope !34137, !noalias !34138
  call void @llvm.lifetime.end.p0(ptr nonnull %23), !noalias !34111
; invoke core::ptr::drop_glue::<core::iter::adapters::filter_map::FilterMap<alloc::vec::into_iter::IntoIter<std::sync::poison::mutex::Mutex<core::option::Option<core::result::Result<(alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)), purrdf_sparql_eval::error::EvalError>>>>, purrdf_sparql_eval::parallel::par_blocks_try_map_init<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_filter<purrdf_core::ir::dataset::RdfDataset>::{closure#0}, purrdf_sparql_eval::expr::eval_filter<purrdf_core::ir::dataset::RdfDataset>::{closure#1}, purrdf_sparql_eval::expr::eval_filter<purrdf_core::ir::dataset::RdfDataset>::{closure#2}>::{closure#2}>>
  invoke fastcc void @core::ptr::drop_glue::<core::iter::adapters::filter_map::FilterMap<alloc::vec::into_iter::IntoIter<std::sync::poison::mutex::Mutex<core::option::Option<core::result::Result<(alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)), purrdf_sparql_eval::error::EvalError>>>>, purrdf_sparql_eval::parallel::par_blocks_try_map_init<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_filter<purrdf_core::ir::dataset::RdfDataset>::{closure#0}, purrdf_sparql_eval::expr::eval_filter<purrdf_core::ir::dataset::RdfDataset>::{closure#1}, purrdf_sparql_eval::expr::eval_filter<purrdf_core::ir::dataset::RdfDataset>::{closure#2}>::{closure#2}>>(ptr noalias nofree noundef nonnull align 8 dereferenceable(32) %27)
          to label %944 unwind label %816, !noalias !34069, !inline_history !34059

944:                                              ; preds = %939
  call void @llvm.lifetime.end.p0(ptr nonnull %27), !noalias !34045
; invoke purrdf_sparql_eval::parallel::concatenate_chunks::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)>
  invoke fastcc void @purrdf_sparql_eval::parallel::concatenate_chunks::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)>(ptr noalias nofree noundef nonnull align 16 captures(none) dereferenceable(256) %117, ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %28)
          to label %945 unwind label %816, !inline_history !34059

945:                                              ; preds = %944
  call void @llvm.lifetime.end.p0(ptr nonnull %28), !noalias !34045
  call void @llvm.lifetime.end.p0(ptr nonnull %30), !noalias !34045
  call void @llvm.lifetime.end.p0(ptr nonnull %31), !noalias !34045
  %946 = load i64, ptr %33, align 8, !noalias !34045
  %947 = icmp eq i64 %946, 0
  br i1 %947, label %951, label %948

948:                                              ; preds = %945
  %949 = load ptr, ptr %712, align 8, !noalias !34045, !nonnull !1708, !noundef !1708
  %950 = shl nuw i64 %946, 4
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %949, i64 noundef %950, i64 noundef range(i64 1, -9223372036854775807) 8) #88, !noalias !34069, !inline_history !34059
  br label %951

951:                                              ; preds = %948, %945
  call void @llvm.lifetime.end.p0(ptr nonnull %33), !noalias !34045
  br label %956

952:                                              ; preds = %.loopexit287
  %953 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<alloc::vec::Vec<std::sync::poison::mutex::Mutex<core::option::Option<core::result::Result<(alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)), purrdf_sparql_eval::error::EvalError>>>>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<std::sync::poison::mutex::Mutex<core::option::Option<core::result::Result<(alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)), purrdf_sparql_eval::error::EvalError>>>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %31) #89
          to label %779 unwind label %954, !noalias !34069, !inline_history !34059

954:                                              ; preds = %952
  %955 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #86, !noalias !34069, !inline_history !34059
  unreachable

956:                                              ; preds = %951, %690
  call void @llvm.lifetime.end.p0(ptr nonnull %36)
  br label %957

957:                                              ; preds = %956, %681
  call void @llvm.lifetime.end.p0(ptr nonnull %115)
  call void @llvm.lifetime.end.p0(ptr nonnull %116)
  %958 = load i64, ptr %117, align 16, !range !2062, !noundef !1708
  %959 = icmp eq i64 %958, -1
  br i1 %959, label %960, label %966

960:                                              ; preds = %957
  %961 = getelementptr inbounds nuw i8, ptr %117, i64 16
  %962 = getelementptr inbounds nuw i8, ptr %117, i64 48
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(64) %118, ptr noundef nonnull align 16 dereferenceable(64) %962, i64 64, i1 false)
  %963 = getelementptr inbounds nuw i8, ptr %0, i64 48
  %964 = getelementptr inbounds nuw i8, ptr %0, i64 16
  %965 = load <4 x i64>, ptr %961, align 16
  call void @llvm.lifetime.end.p0(ptr nonnull %117)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(64) %963, ptr noundef nonnull align 16 dereferenceable(64) %118, i64 64, i1 false)
  store <4 x i64> %965, ptr %964, align 16
  store i64 1, ptr %0, align 16
  call void @llvm.lifetime.end.p0(ptr nonnull %118)
  br label %2981

966:                                              ; preds = %957
  %967 = getelementptr inbounds nuw i8, ptr %117, i64 8
  %968 = getelementptr inbounds nuw i8, ptr %117, i64 24
  %969 = load i64, ptr %968, align 8
  %970 = getelementptr inbounds nuw i8, ptr %117, i64 32
  %971 = load i64, ptr %970, align 16
  %972 = getelementptr inbounds nuw i8, ptr %117, i64 40
  %973 = load i64, ptr %972, align 8
  %974 = getelementptr inbounds nuw i8, ptr %117, i64 48
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(208) %118, ptr noundef nonnull align 16 dereferenceable(208) %974, i64 208, i1 false)
  %975 = getelementptr inbounds nuw i8, ptr %111, i64 24
  %976 = getelementptr inbounds nuw i8, ptr %119, i64 8
  %977 = load <2 x i64>, ptr %967, align 8
  call void @llvm.lifetime.end.p0(ptr nonnull %117)
  call void @llvm.lifetime.start.p0(ptr nonnull %111)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(208) %975, ptr noundef nonnull align 16 dereferenceable(208) %118, i64 208, i1 false)
  call void @llvm.lifetime.start.p0(ptr nonnull %119)
  store i64 %958, ptr %119, align 8
  store <2 x i64> %977, ptr %976, align 8
  call void @llvm.lifetime.end.p0(ptr nonnull %118)
  call void @llvm.lifetime.start.p0(ptr nonnull %112)
  %978 = add i64 %969, -3
  %979 = icmp ult i64 %978, -2
  %980 = select i1 %979, i64 %973, i64 %969
  %981 = add i64 %980, -1
  %982 = select i1 %979, i64 %969, i64 1
  %983 = select i1 %979, i64 1, i64 %973
  store i64 %982, ptr %111, align 8
  %984 = getelementptr inbounds nuw i8, ptr %111, i64 8
  store i64 %971, ptr %984, align 8
  %985 = getelementptr inbounds nuw i8, ptr %111, i64 16
  store i64 %983, ptr %985, align 8
  %986 = getelementptr inbounds nuw i8, ptr %111, i64 232
  store i64 0, ptr %986, align 8
  %987 = getelementptr inbounds nuw i8, ptr %111, i64 240
  store i64 %981, ptr %987, align 8
; invoke <purrdf_core::small::IntoIter<[(purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint); 1]> as core::iter::traits::iterator::Iterator>::unzip::<purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>
  invoke fastcc void @<purrdf_core::small::IntoIter<[(purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint); 1]> as core::iter::traits::iterator::Iterator>::unzip::<purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>(ptr noalias nofree noundef align 8 captures(none) dereferenceable(240) %112, ptr noalias nofree noundef align 8 captures(address) dereferenceable(248) %111)
          to label %990 unwind label %2990

988:                                              ; preds = %2384
  %989 = landingpad { ptr, i32 }
          cleanup
  br label %566

990:                                              ; preds = %966
  call void @llvm.lifetime.end.p0(ptr nonnull %111)
  call void @llvm.lifetime.start.p0(ptr nonnull %114)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %114, ptr noundef nonnull align 8 dereferenceable(32) %112, i64 32, i1 false)
  call void @llvm.lifetime.start.p0(ptr nonnull %113)
  %991 = getelementptr inbounds nuw i8, ptr %112, i64 32
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(208) %113, ptr noundef nonnull align 8 dereferenceable(208) %991, i64 208, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %112)
  call void @llvm.lifetime.start.p0(ptr nonnull %109)
  call void @llvm.lifetime.start.p0(ptr nonnull %108)
  call void @llvm.lifetime.start.p0(ptr nonnull %107)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %107, ptr noundef nonnull align 8 dereferenceable(24) %119, i64 24, i1 false)
  call void @llvm.experimental.noalias.scope.decl(metadata !34139)
  call void @llvm.experimental.noalias.scope.decl(metadata !34142)
  call void @llvm.experimental.noalias.scope.decl(metadata !34144)
  call void @llvm.experimental.noalias.scope.decl(metadata !34146)
  %992 = getelementptr inbounds nuw i8, ptr %123, i64 194
  %993 = load i8, ptr %992, align 2, !range !3634, !alias.scope !34139, !noalias !34148, !noundef !1708
  %994 = icmp ne i8 %993, 2
  %995 = load ptr, ptr %555, align 8, !alias.scope !34142, !noalias !34150
  %996 = icmp eq ptr %995, null
  %997 = select i1 %994, i1 true, i1 %996
  br i1 %997, label %998, label %1001

998:                                              ; preds = %990
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %108, ptr noundef nonnull align 8 dereferenceable(24) %119, i64 24, i1 false)
; invoke core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(208) %113)
          to label %2394 unwind label %999

999:                                              ; preds = %998
  %1000 = landingpad { ptr, i32 }
          cleanup
  br label %2988

1001:                                             ; preds = %990
  call void @llvm.experimental.noalias.scope.decl(metadata !34151)
  call void @llvm.experimental.noalias.scope.decl(metadata !34154)
  call void @llvm.experimental.noalias.scope.decl(metadata !34156)
  call void @llvm.experimental.noalias.scope.decl(metadata !34158)
  call void @llvm.lifetime.start.p0(ptr nonnull %89), !noalias !34160
  call void @llvm.lifetime.start.p0(ptr nonnull %88), !noalias !34160
  %1002 = load i64, ptr %113, align 8, !alias.scope !34162, !noalias !34163
  %1003 = getelementptr inbounds nuw i8, ptr %113, i64 8
  %1004 = load i64, ptr %1003, align 8, !alias.scope !34162, !noalias !34163
  %1005 = getelementptr inbounds nuw i8, ptr %113, i64 16
  %1006 = load i64, ptr %1005, align 8, !alias.scope !34162, !noalias !34163
  %1007 = getelementptr inbounds nuw i8, ptr %113, i64 24
  %1008 = getelementptr inbounds nuw i8, ptr %88, i64 24
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(184) %1008, ptr noundef nonnull readonly align 8 dereferenceable(184) %1007, i64 184, i1 false), !noalias !34163
  call void @llvm.experimental.noalias.scope.decl(metadata !34164)
  %1009 = icmp ugt i64 %1002, 2
  %1010 = select i1 %1009, i64 %1006, i64 %1002
  %1011 = add i64 %1010, -1
  %1012 = select i1 %1009, i64 %1002, i64 1
  %1013 = select i1 %1009, i64 1, i64 %1006
  store i64 %1012, ptr %88, align 8, !alias.scope !34167, !noalias !34160
  %1014 = getelementptr inbounds nuw i8, ptr %88, i64 8
  store i64 %1004, ptr %1014, align 8, !alias.scope !34167, !noalias !34160
  %1015 = getelementptr inbounds nuw i8, ptr %88, i64 16
  store i64 %1013, ptr %1015, align 8, !alias.scope !34167, !noalias !34160
  %1016 = getelementptr inbounds nuw i8, ptr %88, i64 208
  store i64 0, ptr %1016, align 8, !alias.scope !34169, !noalias !34170
  %1017 = getelementptr inbounds nuw i8, ptr %88, i64 216
  store i64 %1011, ptr %1017, align 8, !alias.scope !34169, !noalias !34170
; invoke <purrdf_core::small::IntoIter<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]> as core::iter::traits::iterator::Iterator>::collect::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>
  invoke fastcc void @<purrdf_core::small::IntoIter<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]> as core::iter::traits::iterator::Iterator>::collect::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>(ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %89, ptr noalias nofree noundef align 8 captures(address) dereferenceable(224) %88)
          to label %1020 unwind label %1018, !noalias !34171

1018:                                             ; preds = %1001
  %1019 = landingpad { ptr, i32 }
          cleanup
  br label %2382

1020:                                             ; preds = %1001
  call void @llvm.lifetime.end.p0(ptr nonnull %88), !noalias !34160
  %1021 = getelementptr inbounds nuw i8, ptr %89, i64 8
  %1022 = load ptr, ptr %1021, align 8, !noalias !34160, !nonnull !1708, !noundef !1708
  %1023 = getelementptr inbounds nuw i8, ptr %89, i64 16
  %1024 = load i64, ptr %1023, align 8, !noalias !34160, !noundef !1708
  %1025 = mul nuw nsw i64 %1024, 200
  %1026 = getelementptr inbounds nuw i8, ptr %1022, i64 %1025
  %1027 = icmp eq i64 %1024, 0
  br i1 %1027, label %1096, label %.preheader279.preheader

.preheader279.preheader:                          ; preds = %1020
  %xtraiter2266 = and i64 %1024, 3
  %1028 = icmp ult i64 %1024, 4
  br i1 %1028, label %.preheader279.epil.preheader, label %.preheader279.preheader.new

.preheader279.preheader.new:                      ; preds = %.preheader279.preheader
  %unroll_iter2275 = and i64 %1024, -4
  br label %.preheader279

.preheader279:                                    ; preds = %1073, %.preheader279.preheader.new
  %1029 = phi i64 [ 0, %.preheader279.preheader.new ], [ %1076, %1073 ]
  %1030 = phi i64 [ 0, %.preheader279.preheader.new ], [ %1075, %1073 ]
  %niter2276 = phi i64 [ 0, %.preheader279.preheader.new ], [ %niter2276.next.3, %1073 ]
  %1031 = getelementptr inbounds nuw [200 x i8], ptr %1022, i64 %1029
  %1032 = getelementptr i8, ptr %1031, i64 168
  %1033 = load i64, ptr %1032, align 8, !noalias !34171, !noundef !1708
  %1034 = getelementptr i8, ptr %1031, i64 176
  %1035 = load i64, ptr %1034, align 8, !noalias !34171, !noundef !1708
  %1036 = call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %1035, i64 %1033)
  %1037 = extractvalue { i64, i1 } %1036, 0
  %1038 = extractvalue { i64, i1 } %1036, 1
  br i1 %1038, label %1039, label %.preheader279.1, !prof !1803

1039:                                             ; preds = %.preheader279
  br label %.preheader279.1

.preheader279.1:                                  ; preds = %1039, %.preheader279
  %1040 = phi i64 [ -1, %1039 ], [ %1037, %.preheader279 ]
  %1041 = call noundef i64 @llvm.uadd.sat.i64(i64 %1030, i64 %1040)
  %1042 = getelementptr inbounds nuw [200 x i8], ptr %1022, i64 %1029
  %1043 = getelementptr i8, ptr %1042, i64 368
  %1044 = load i64, ptr %1043, align 8, !noalias !34171, !noundef !1708
  %1045 = getelementptr i8, ptr %1042, i64 376
  %1046 = load i64, ptr %1045, align 8, !noalias !34171, !noundef !1708
  %1047 = call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %1046, i64 %1044)
  %1048 = extractvalue { i64, i1 } %1047, 0
  %1049 = extractvalue { i64, i1 } %1047, 1
  br i1 %1049, label %1050, label %.preheader279.2, !prof !1803

1050:                                             ; preds = %.preheader279.1
  br label %.preheader279.2

.preheader279.2:                                  ; preds = %1050, %.preheader279.1
  %1051 = phi i64 [ -1, %1050 ], [ %1048, %.preheader279.1 ]
  %1052 = call noundef i64 @llvm.uadd.sat.i64(i64 %1041, i64 %1051)
  %1053 = getelementptr inbounds nuw [200 x i8], ptr %1022, i64 %1029
  %1054 = getelementptr i8, ptr %1053, i64 568
  %1055 = load i64, ptr %1054, align 8, !noalias !34171, !noundef !1708
  %1056 = getelementptr i8, ptr %1053, i64 576
  %1057 = load i64, ptr %1056, align 8, !noalias !34171, !noundef !1708
  %1058 = call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %1057, i64 %1055)
  %1059 = extractvalue { i64, i1 } %1058, 0
  %1060 = extractvalue { i64, i1 } %1058, 1
  br i1 %1060, label %1061, label %.preheader279.3, !prof !1803

1061:                                             ; preds = %.preheader279.2
  br label %.preheader279.3

.preheader279.3:                                  ; preds = %1061, %.preheader279.2
  %1062 = phi i64 [ -1, %1061 ], [ %1059, %.preheader279.2 ]
  %1063 = call noundef i64 @llvm.uadd.sat.i64(i64 %1052, i64 %1062)
  %1064 = getelementptr inbounds nuw [200 x i8], ptr %1022, i64 %1029
  %1065 = getelementptr i8, ptr %1064, i64 768
  %1066 = load i64, ptr %1065, align 8, !noalias !34171, !noundef !1708
  %1067 = getelementptr i8, ptr %1064, i64 776
  %1068 = load i64, ptr %1067, align 8, !noalias !34171, !noundef !1708
  %1069 = call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %1068, i64 %1066)
  %1070 = extractvalue { i64, i1 } %1069, 0
  %1071 = extractvalue { i64, i1 } %1069, 1
  br i1 %1071, label %1072, label %1073, !prof !1803

1072:                                             ; preds = %.preheader279.3
  br label %1073

1073:                                             ; preds = %1072, %.preheader279.3
  %1074 = phi i64 [ -1, %1072 ], [ %1070, %.preheader279.3 ]
  %1075 = call noundef i64 @llvm.uadd.sat.i64(i64 %1063, i64 %1074)
  %1076 = add nuw i64 %1029, 4
  %niter2276.next.3 = add i64 %niter2276, 4
  %niter2276.ncmp.3 = icmp eq i64 %niter2276.next.3, %unroll_iter2275
  br i1 %niter2276.ncmp.3, label %.unr-lcssa, label %.preheader279

.unr-lcssa:                                       ; preds = %1073
  %lcmp.mod2272.not = icmp eq i64 %xtraiter2266, 0
  br i1 %lcmp.mod2272.not, label %.epilog-lcssa, label %.preheader279.epil.preheader

.preheader279.epil.preheader:                     ; preds = %.unr-lcssa, %.preheader279.preheader
  %.epil.init2269 = phi i64 [ 0, %.preheader279.preheader ], [ %1076, %.unr-lcssa ]
  %.epil.init2271 = phi i64 [ 0, %.preheader279.preheader ], [ %1075, %.unr-lcssa ]
  %lcmp.mod2274 = icmp ne i64 %xtraiter2266, 0
  call void @llvm.assume(i1 %lcmp.mod2274)
  br label %.preheader279.epil

.preheader279.epil:                               ; preds = %1088, %.preheader279.epil.preheader
  %1077 = phi i64 [ %1091, %1088 ], [ %.epil.init2269, %.preheader279.epil.preheader ]
  %1078 = phi i64 [ %1090, %1088 ], [ %.epil.init2271, %.preheader279.epil.preheader ]
  %epil.iter2267 = phi i64 [ %epil.iter2267.next, %1088 ], [ 0, %.preheader279.epil.preheader ]
  %1079 = getelementptr inbounds nuw [200 x i8], ptr %1022, i64 %1077
  %1080 = getelementptr i8, ptr %1079, i64 168
  %1081 = load i64, ptr %1080, align 8, !noalias !34171, !noundef !1708
  %1082 = getelementptr i8, ptr %1079, i64 176
  %1083 = load i64, ptr %1082, align 8, !noalias !34171, !noundef !1708
  %1084 = call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %1083, i64 %1081)
  %1085 = extractvalue { i64, i1 } %1084, 0
  %1086 = extractvalue { i64, i1 } %1084, 1
  br i1 %1086, label %1087, label %1088, !prof !1803

1087:                                             ; preds = %.preheader279.epil
  br label %1088

1088:                                             ; preds = %1087, %.preheader279.epil
  %1089 = phi i64 [ -1, %1087 ], [ %1085, %.preheader279.epil ]
  %1090 = call noundef i64 @llvm.uadd.sat.i64(i64 %1078, i64 %1089)
  %1091 = add nuw i64 %1077, 1
  %epil.iter2267.next = add i64 %epil.iter2267, 1
  %epil.iter2267.cmp.not = icmp eq i64 %epil.iter2267.next, %xtraiter2266
  br i1 %epil.iter2267.cmp.not, label %.epilog-lcssa, label %.preheader279.epil, !llvm.loop !34172

.epilog-lcssa:                                    ; preds = %1088, %.unr-lcssa
  %.lcssa2211 = phi i64 [ %1075, %.unr-lcssa ], [ %1090, %1088 ]
  %1092 = load ptr, ptr %555, align 8, !alias.scope !34173, !noalias !34171, !noundef !1708
  %1093 = icmp eq ptr %1092, null
  %1094 = icmp eq i64 %.lcssa2211, 0
  %1095 = or i1 %1094, %1093
  br i1 %1095, label %1096, label %1148

1096:                                             ; preds = %1152, %1148, %.epilog-lcssa, %1020
  %1097 = load i64, ptr %89, align 8, !range !1817, !noalias !34160, !noundef !1708
  %1098 = icmp ult i64 %1024, 46116860184273880
  call void @llvm.assume(i1 %1098)
  call void @llvm.lifetime.start.p0(ptr nonnull %82), !noalias !34174
  store i64 0, ptr %82, align 8, !noalias !34174
  %1099 = getelementptr inbounds nuw i8, ptr %82, i64 8
  store ptr inttoptr (i64 8 to ptr), ptr %1099, align 8, !noalias !34174
  %1100 = getelementptr inbounds nuw i8, ptr %82, i64 16
  store i64 0, ptr %1100, align 8, !noalias !34174
  call void @llvm.lifetime.start.p0(ptr nonnull %81), !noalias !34174
  store ptr %1022, ptr %81, align 8, !noalias !34178
  %1101 = getelementptr inbounds nuw i8, ptr %81, i64 8
  %1102 = getelementptr inbounds nuw i8, ptr %81, i64 16
  store i64 %1097, ptr %1102, align 8, !noalias !34178
  %1103 = getelementptr inbounds nuw i8, ptr %81, i64 24
  store ptr %1026, ptr %1103, align 8, !noalias !34178
  br i1 %1027, label %.loopexit275, label %1104

1104:                                             ; preds = %1096
  %1105 = getelementptr inbounds nuw i8, ptr %80, i64 8
  %1106 = getelementptr inbounds nuw i8, ptr %80, i64 152
  br label %1111

1107:                                             ; preds = %1118, %1109
  %1108 = phi { ptr, i32 } [ %1110, %1109 ], [ %1128, %1118 ]
; invoke core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>(ptr noalias nofree noundef align 8 dereferenceable(24) %82) #89
          to label %2382 unwind label %1146, !noalias !34179

1109:                                             ; preds = %1134
  %1110 = landingpad { ptr, i32 }
          cleanup
  br label %1107

1111:                                             ; preds = %1143, %1104
  %1112 = phi ptr [ inttoptr (i64 8 to ptr), %1104 ], [ %1139, %1143 ]
  %1113 = phi i64 [ 0, %1104 ], [ %1141, %1143 ]
  %1114 = phi ptr [ %1022, %1104 ], [ %1115, %1143 ]
  %1115 = getelementptr inbounds nuw i8, ptr %1114, i64 200
  %1116 = load i64, ptr %1114, align 8, !noalias !34180
  %1117 = icmp eq i64 %1116, -1
  br i1 %1117, label %.loopexit275, label %1119

1118:                                             ; preds = %1127
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(32) %81)
          to label %1107 unwind label %1146, !noalias !34179

1119:                                             ; preds = %1111
  %1120 = getelementptr inbounds nuw i8, ptr %1114, i64 8
  call void @llvm.lifetime.start.p0(ptr nonnull %80), !noalias !34174
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(152) %1105, ptr noundef nonnull align 8 dereferenceable(152) %1120, i64 152, i1 false), !noalias !34179
  store i64 %1116, ptr %80, align 8, !noalias !34174
  %1121 = load i8, ptr %1106, align 8, !range !1746, !noalias !34174, !noundef !1708
  call void @llvm.experimental.noalias.scope.decl(metadata !34186)
  %1122 = load i64, ptr %82, align 8, !range !1817, !alias.scope !34186, !noalias !34189, !noundef !1708
  %1123 = icmp eq i64 %1113, %1122
  br i1 %1123, label %1124, label %1138

1124:                                             ; preds = %1119
; invoke <alloc::raw_vec::RawVec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>::grow_one
  invoke void @<alloc::raw_vec::RawVec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>::grow_one(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %82)
          to label %1125 unwind label %1127, !noalias !34191

1125:                                             ; preds = %1124
  %1126 = load ptr, ptr %1099, align 8, !alias.scope !34186, !noalias !34189
  br label %1138

1127:                                             ; preds = %1124
  %1128 = landingpad { ptr, i32 }
          cleanup
  store ptr %1115, ptr %1101, align 8, !noalias !34174
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(160) %80) #89
          to label %1118 unwind label %1129, !noalias !34192

1129:                                             ; preds = %1127
  %1130 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #86, !noalias !34193
  unreachable

.loopexit275:                                     ; preds = %1143, %1111, %1096
  %1131 = phi i64 [ 0, %1096 ], [ %1141, %1143 ], [ %1113, %1111 ]
  %1132 = phi ptr [ inttoptr (i64 8 to ptr), %1096 ], [ %1139, %1143 ], [ %1112, %1111 ]
  %1133 = phi ptr [ %1022, %1096 ], [ %1026, %1143 ], [ %1115, %1111 ]
  store ptr %1133, ptr %1101, align 8, !noalias !34174
  br label %1134

1134:                                             ; preds = %1145, %.loopexit275
  %1135 = phi i64 [ %1141, %1145 ], [ %1131, %.loopexit275 ]
  %1136 = phi ptr [ %1139, %1145 ], [ %1132, %.loopexit275 ]
  %1137 = phi i8 [ 1, %1145 ], [ 0, %.loopexit275 ]
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(32) %81)
          to label %1155 unwind label %1109, !noalias !34179

1138:                                             ; preds = %1125, %1119
  %1139 = phi ptr [ %1126, %1125 ], [ %1112, %1119 ]
  %1140 = getelementptr inbounds nuw [160 x i8], ptr %1139, i64 %1113
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(160) %1140, ptr noundef nonnull readonly align 8 dereferenceable(160) %80, i64 160, i1 false), !noalias !34192
  %1141 = add nuw nsw i64 %1113, 1
  store i64 %1141, ptr %1100, align 8, !alias.scope !34186, !noalias !34189
  %1142 = trunc nuw i8 %1121 to i1
  br i1 %1142, label %1145, label %1143

1143:                                             ; preds = %1138
  call void @llvm.lifetime.end.p0(ptr nonnull %80), !noalias !34174
  %1144 = icmp eq ptr %1115, %1026
  br i1 %1144, label %.loopexit275, label %1111

1145:                                             ; preds = %1138
  store ptr %1115, ptr %1101, align 8, !noalias !34174
  call void @llvm.lifetime.end.p0(ptr nonnull %80), !noalias !34174
  br label %1134

1146:                                             ; preds = %1118, %1107
  %1147 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #86, !noalias !34179
  unreachable

1148:                                             ; preds = %.epilog-lcssa
  %1149 = getelementptr inbounds nuw i8, ptr %1092, i64 336
  %1150 = load ptr, ptr %1149, align 8, !noalias !34171, !noundef !1708
  %1151 = icmp eq ptr %1150, null
  br i1 %1151, label %1096, label %1152

1152:                                             ; preds = %1148
  %1153 = getelementptr inbounds nuw i8, ptr %1092, i64 352
  %1154 = atomicrmw add ptr %1153, i64 %.lcssa2211 monotonic, align 8, !noalias !34171
  br label %1096

1155:                                             ; preds = %1134
  call void @llvm.lifetime.end.p0(ptr nonnull %81), !noalias !34174
  %1156 = load i64, ptr %82, align 8, !noalias !34194
  call void @llvm.lifetime.end.p0(ptr nonnull %82), !noalias !34174
  %1157 = getelementptr inbounds nuw i8, ptr %123, i64 193
  %1158 = load i8, ptr %1157, align 1, !range !1746, !alias.scope !34195, !noalias !34196, !noundef !1708
  %1159 = trunc nuw i8 %1158 to i1
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %1136) ]
  %1160 = icmp ne i64 %1135, 0
  br i1 %1160, label %iter.check, label %.loopexit274

iter.check:                                       ; preds = %1155
  %min.iters.check = icmp ult i64 %1135, 8
  br i1 %min.iters.check, label %.preheader273.preheader, label %vector.main.loop.iter.check

vector.main.loop.iter.check:                      ; preds = %iter.check
  %min.iters.check1770 = icmp ult i64 %1135, 32
  br i1 %min.iters.check1770, label %vec.epilog.ph, label %vector.ph

vector.ph:                                        ; preds = %vector.main.loop.iter.check
  %n.mod.vf = and i64 %1135, 24
  %n.vec = and i64 %1135, -32
  br label %vector.body

vector.body:                                      ; preds = %vector.body, %vector.ph
  %index = phi i64 [ 0, %vector.ph ], [ %index.next, %vector.body ]
  %vec.ind = phi <8 x i64> [ <i64 0, i64 1, i64 2, i64 3, i64 4, i64 5, i64 6, i64 7>, %vector.ph ], [ %vec.ind.next, %vector.body ]
  %vec.phi = phi <8 x i64> [ zeroinitializer, %vector.ph ], [ %1161, %vector.body ]
  %vec.phi1771 = phi <8 x i64> [ zeroinitializer, %vector.ph ], [ %1162, %vector.body ]
  %vec.phi1772 = phi <8 x i64> [ zeroinitializer, %vector.ph ], [ %1163, %vector.body ]
  %vec.phi1773 = phi <8 x i64> [ zeroinitializer, %vector.ph ], [ %1164, %vector.body ]
  %step.add = add nuw <8 x i64> %vec.ind, splat (i64 8)
  %step.add.2 = add nuw <8 x i64> %vec.ind, splat (i64 16)
  %step.add.3 = add nuw <8 x i64> %vec.ind, splat (i64 24)
  %wide.gep = getelementptr inbounds nuw [160 x i8], ptr %1136, <8 x i64> %vec.ind
  %wide.gep1774 = getelementptr inbounds nuw [160 x i8], ptr %1136, <8 x i64> %step.add
  %wide.gep1775 = getelementptr inbounds nuw [160 x i8], ptr %1136, <8 x i64> %step.add.2
  %wide.gep1776 = getelementptr inbounds nuw [160 x i8], ptr %1136, <8 x i64> %step.add.3
  %wide.gep1777 = getelementptr i8, <8 x ptr> %wide.gep, i64 64
  %wide.gep1778 = getelementptr i8, <8 x ptr> %wide.gep1774, i64 64
  %wide.gep1779 = getelementptr i8, <8 x ptr> %wide.gep1775, i64 64
  %wide.gep1780 = getelementptr i8, <8 x ptr> %wide.gep1776, i64 64
  %wide.masked.gather = call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep1777, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !34197
  %wide.masked.gather1781 = call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep1778, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !34197
  %wide.masked.gather1782 = call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep1779, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !34197
  %wide.masked.gather1783 = call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep1780, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !34197
  %1161 = add <8 x i64> %wide.masked.gather, %vec.phi
  %1162 = add <8 x i64> %wide.masked.gather1781, %vec.phi1771
  %1163 = add <8 x i64> %wide.masked.gather1782, %vec.phi1772
  %1164 = add <8 x i64> %wide.masked.gather1783, %vec.phi1773
  %index.next = add nuw i64 %index, 32
  %vec.ind.next = add nuw <8 x i64> %vec.ind, splat (i64 32)
  %1165 = icmp eq i64 %index.next, %n.vec
  br i1 %1165, label %middle.block, label %vector.body, !llvm.loop !34200

middle.block:                                     ; preds = %vector.body
  %bin.rdx = add <8 x i64> %1162, %1161
  %bin.rdx1784 = add <8 x i64> %1163, %bin.rdx
  %bin.rdx1785 = add <8 x i64> %1164, %bin.rdx1784
  %1166 = call i64 @llvm.vector.reduce.add.v8i64(<8 x i64> %bin.rdx1785)
  %cmp.n = icmp eq i64 %1135, %n.vec
  br i1 %cmp.n, label %.loopexit274, label %vec.epilog.iter.check

vec.epilog.iter.check:                            ; preds = %middle.block
  %min.epilog.iters.check = icmp eq i64 %n.mod.vf, 0
  br i1 %min.epilog.iters.check, label %.preheader273.preheader, label %vec.epilog.ph, !prof !29315

vec.epilog.ph:                                    ; preds = %vector.main.loop.iter.check, %vec.epilog.iter.check
  %vec.epilog.resume.val = phi i64 [ %n.vec, %vec.epilog.iter.check ], [ 0, %vector.main.loop.iter.check ]
  %bc.merge.rdx = phi i64 [ %1166, %vec.epilog.iter.check ], [ 0, %vector.main.loop.iter.check ]
  %n.vec1787 = and i64 %1135, -8
  %1167 = insertelement <8 x i64> <i64 poison, i64 0, i64 0, i64 0, i64 0, i64 0, i64 0, i64 0>, i64 %bc.merge.rdx, i64 0
  %broadcast.splatinsert = insertelement <8 x i64> poison, i64 %vec.epilog.resume.val, i64 0
  %broadcast.splat = shufflevector <8 x i64> %broadcast.splatinsert, <8 x i64> poison, <8 x i32> zeroinitializer
  %induction = or disjoint <8 x i64> %broadcast.splat, <i64 0, i64 1, i64 2, i64 3, i64 4, i64 5, i64 6, i64 7>
  br label %vec.epilog.vector.body

vec.epilog.vector.body:                           ; preds = %vec.epilog.vector.body, %vec.epilog.ph
  %index1788 = phi i64 [ %vec.epilog.resume.val, %vec.epilog.ph ], [ %index.next1794, %vec.epilog.vector.body ]
  %vec.ind1789 = phi <8 x i64> [ %induction, %vec.epilog.ph ], [ %vec.ind.next1795, %vec.epilog.vector.body ]
  %vec.phi1790 = phi <8 x i64> [ %1167, %vec.epilog.ph ], [ %1168, %vec.epilog.vector.body ]
  %wide.gep1791 = getelementptr inbounds nuw [160 x i8], ptr %1136, <8 x i64> %vec.ind1789
  %wide.gep1792 = getelementptr i8, <8 x ptr> %wide.gep1791, i64 64
  %wide.masked.gather1793 = call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep1792, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !34197
  %1168 = add <8 x i64> %wide.masked.gather1793, %vec.phi1790
  %index.next1794 = add nuw i64 %index1788, 8
  %vec.ind.next1795 = add nuw <8 x i64> %vec.ind1789, splat (i64 8)
  %1169 = icmp eq i64 %index.next1794, %n.vec1787
  br i1 %1169, label %vec.epilog.middle.block, label %vec.epilog.vector.body, !llvm.loop !34201

vec.epilog.middle.block:                          ; preds = %vec.epilog.vector.body
  %1170 = call i64 @llvm.vector.reduce.add.v8i64(<8 x i64> %1168)
  %cmp.n1796 = icmp eq i64 %1135, %n.vec1787
  br i1 %cmp.n1796, label %.loopexit274, label %.preheader273.preheader

.preheader273.preheader:                          ; preds = %iter.check, %vec.epilog.iter.check, %vec.epilog.middle.block
  %.ph2195 = phi i64 [ 0, %iter.check ], [ %n.vec, %vec.epilog.iter.check ], [ %n.vec1787, %vec.epilog.middle.block ]
  %.ph2196 = phi i64 [ 0, %iter.check ], [ %1166, %vec.epilog.iter.check ], [ %1170, %vec.epilog.middle.block ]
  br label %.preheader273

.preheader273:                                    ; preds = %.preheader273.preheader, %.preheader273
  %1171 = phi i64 [ %1178, %.preheader273 ], [ %.ph2195, %.preheader273.preheader ]
  %1172 = phi i64 [ %1177, %.preheader273 ], [ %.ph2196, %.preheader273.preheader ]
  %1173 = getelementptr inbounds nuw [160 x i8], ptr %1136, i64 %1171
  %1174 = getelementptr i8, ptr %1173, i64 64
  %1175 = load i64, ptr %1174, align 8, !noalias !34197, !noundef !1708
  %1176 = icmp ult i64 %1175, 288230376151711744
  call void @llvm.assume(i1 %1176)
  %1177 = add i64 %1175, %1172
  %1178 = add nuw i64 %1171, 1
  %1179 = icmp eq i64 %1178, %1135
  br i1 %1179, label %.loopexit274, label %.preheader273, !llvm.loop !34202

.loopexit274:                                     ; preds = %.preheader273, %middle.block, %vec.epilog.middle.block, %1155
  %1180 = phi i64 [ 0, %1155 ], [ %1170, %vec.epilog.middle.block ], [ %1166, %middle.block ], [ %1177, %.preheader273 ]
  %1181 = trunc nuw i8 %1137 to i1
  call void @llvm.lifetime.start.p0(ptr nonnull %87)
  call void @llvm.lifetime.start.p0(ptr nonnull %85)
  call void @llvm.lifetime.start.p0(ptr nonnull %86)
  %1182 = getelementptr inbounds nuw i8, ptr %123, i64 195
  %1183 = load i8, ptr %1182, align 1, !range !10412, !alias.scope !34195, !noalias !34196, !noundef !1708
  call void @llvm.lifetime.start.p0(ptr nonnull %84), !noalias !34160
  store i64 %1156, ptr %84, align 8, !noalias !34160
  %1184 = getelementptr inbounds nuw i8, ptr %84, i64 8
  store ptr %1136, ptr %1184, align 8, !noalias !34160
  %1185 = getelementptr inbounds nuw i8, ptr %84, i64 16
  store i64 %1135, ptr %1185, align 8, !noalias !34160
  %1186 = getelementptr inbounds nuw i8, ptr %84, i64 24
  store i8 %1137, ptr %1186, align 8, !noalias !34160
  call void @llvm.experimental.noalias.scope.decl(metadata !34203)
  call void @llvm.experimental.noalias.scope.decl(metadata !34206)
  call void @llvm.lifetime.start.p0(ptr nonnull %62)
  call void @llvm.lifetime.start.p0(ptr nonnull %73)
  call void @llvm.lifetime.start.p0(ptr nonnull %79), !noalias !34208
  call void @llvm.lifetime.start.p0(ptr nonnull %78)
  call void @llvm.lifetime.start.p0(ptr nonnull %77), !noalias !34208
  call void @llvm.lifetime.start.p0(ptr nonnull %76), !noalias !34208
  store i64 0, ptr %76, align 8, !noalias !34208
  %1187 = getelementptr inbounds nuw i8, ptr %76, i64 8
  store ptr inttoptr (i64 8 to ptr), ptr %1187, align 8, !noalias !34208
  %1188 = getelementptr inbounds nuw i8, ptr %76, i64 16
  store i64 0, ptr %1188, align 8, !noalias !34208
  %1189 = getelementptr inbounds nuw i8, ptr %107, i64 16
  %1190 = load i64, ptr %1189, align 8, !alias.scope !34211, !noalias !34212, !noundef !1708
  %1191 = icmp ult i64 %1190, 230584300921369396
  call void @llvm.assume(i1 %1191)
; invoke purrdf_sparql_eval::parallel::reserve_concatenated_rows::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, [usize; 1]>
  invoke fastcc void @purrdf_sparql_eval::parallel::reserve_concatenated_rows::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, [usize; 1]>(ptr noalias nofree noundef align 16 captures(none) dereferenceable(96) %77, ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %76, i64 noundef %1190)
          to label %1192 unwind label %2326, !noalias !34213

1192:                                             ; preds = %.loopexit274
  call void @llvm.lifetime.end.p0(ptr nonnull %76), !noalias !34208
  %1193 = load i64, ptr %77, align 16, !range !2530, !noalias !34208, !noundef !1708
  %1194 = icmp eq i64 %1193, -1
  %1195 = getelementptr inbounds nuw i8, ptr %77, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %78, ptr noundef nonnull align 8 dereferenceable(24) %1195, i64 24, i1 false), !noalias !34208
  br i1 %1194, label %1253, label %1196

1196:                                             ; preds = %1192
  %1197 = getelementptr inbounds nuw i8, ptr %77, i64 32
  %1198 = load i8, ptr %1197, align 16, !noalias !34214
  %1199 = getelementptr inbounds nuw i8, ptr %77, i64 33
  %1200 = load i56, ptr %1199, align 1, !noalias !34214
  %1201 = getelementptr inbounds nuw i8, ptr %77, i64 40
  %1202 = load i64, ptr %1201, align 8, !noalias !34214
  %1203 = getelementptr inbounds nuw i8, ptr %77, i64 48
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(48) %86, ptr noundef nonnull align 16 dereferenceable(48) %1203, i64 48, i1 false), !noalias !34214
  call void @llvm.lifetime.end.p0(ptr nonnull %77), !noalias !34208
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %85, ptr noundef nonnull align 8 dereferenceable(24) %78, i64 24, i1 false), !noalias !34214
  call void @llvm.lifetime.end.p0(ptr nonnull %78)
  call void @llvm.lifetime.end.p0(ptr nonnull %79), !noalias !34208
  call void @llvm.experimental.noalias.scope.decl(metadata !34215)
  %1204 = getelementptr inbounds nuw i8, ptr %107, i64 8
  %1205 = load ptr, ptr %1204, align 8, !alias.scope !34218, !noalias !34212, !nonnull !1708, !noundef !1708
  call void @llvm.experimental.noalias.scope.decl(metadata !34219)
  %1206 = icmp eq i64 %1190, 0
  br i1 %1206, label %.loopexit272, label %.preheader271

.preheader271:                                    ; preds = %1196
  %1207 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %1208 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  br label %1209

1209:                                             ; preds = %.preheader271, %1247
  %1210 = phi i64 [ %1212, %1247 ], [ 0, %.preheader271 ]
  %1211 = getelementptr inbounds nuw [40 x i8], ptr %1205, i64 %1210
  %1212 = add nuw nsw i64 %1210, 1
  %1213 = load i64, ptr %1211, align 8, !range !1940, !alias.scope !34222, !noalias !34225, !noundef !1708
  %1214 = icmp ugt i64 %1213, 5
  br i1 %1214, label %1215, label %1247

1215:                                             ; preds = %1209
  %1216 = getelementptr i8, ptr %1211, i64 8
  %1217 = load ptr, ptr %1216, align 8, !alias.scope !34219, !noalias !34225, !nonnull !1708, !noundef !1708
  %1218 = shl i64 %1213, 3
  %1219 = add i64 %1218, -8
  %1220 = load i64, ptr %1207, align 8, !noalias !34226, !noundef !1708
  %1221 = call i64 @llvm.umin.i64(i64 %1219, i64 9223372036854775807)
  %1222 = call i64 @llvm.ssub.sat.i64(i64 %1220, i64 %1221)
  store i64 %1222, ptr %1207, align 8, !noalias !34226
  %1223 = load i64, ptr %1208, align 8, !noalias !34226, !noundef !1708
  %1224 = icmp slt i64 %1222, %1223
  br i1 %1224, label %1225, label %.preheader2194

1225:                                             ; preds = %1215
  store i64 %1222, ptr %1208, align 8, !noalias !34226
  br label %.preheader2194

.preheader2194:                                   ; preds = %1225, %1215
  br label %1226

1226:                                             ; preds = %.preheader2194, %1229
  %1227 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952) acquire, align 8, !noalias !34226
  %1228 = icmp slt i64 %1227, 0
  br i1 %1228, label %1229, label %__rustc::__rust_dealloc (.exit217)

1229:                                             ; preds = %1226
  %1230 = add nsw i64 %1227, 1
  %1231 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 %1227, i64 %1230 acq_rel acquire, align 8, !noalias !34226
  %1232 = extractvalue { i64, i1 } %1231, 1
  br i1 %1232, label %1233, label %1226

1233:                                             ; preds = %1229
  %1234 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1221 monotonic, align 8, !noalias !34226
  %1235 = call i64 @llvm.ssub.sat.i64(i64 %1234, i64 %1221)
  %1236 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !34226
  br label %1237

1237:                                             ; preds = %1240, %1233
  %1238 = phi i64 [ %1236, %1233 ], [ %1243, %1240 ]
  %1239 = icmp slt i64 %1235, %1238
  br i1 %1239, label %1240, label %1244

1240:                                             ; preds = %1237
  %1241 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1238, i64 %1235 monotonic monotonic, align 8, !noalias !34226
  %1242 = extractvalue { i64, i1 } %1241, 1
  %1243 = extractvalue { i64, i1 } %1241, 0
  br i1 %1242, label %1244, label %1237

1244:                                             ; preds = %1240, %1237
  %1245 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 1 release, align 8, !noalias !34226
  br label %__rustc::__rust_dealloc (.exit217)

__rustc::__rust_dealloc (.exit217): ; preds = %1226, %1244
  %1246 = icmp ne i64 %1219, 0
  call void @llvm.assume(i1 %1246), !noalias !34226
  call void @free(ptr noundef nonnull %1217) #88, !noalias !34226
  br label %1247

1247:                                             ; preds = %__rustc::__rust_dealloc (.exit217), %1209
  %1248 = icmp eq i64 %1212, %1190
  br i1 %1248, label %.loopexit272, label %1209

.loopexit272:                                     ; preds = %1247, %1196
  %1249 = load i64, ptr %107, align 8, !alias.scope !34218, !noalias !34212
  %1250 = icmp eq i64 %1249, 0
  br i1 %1250, label %2271, label %1251

1251:                                             ; preds = %.loopexit272
  %1252 = mul nuw i64 %1249, 40
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %1205, i64 noundef %1252, i64 noundef range(i64 1, -9223372036854775807) 8) #88, !noalias !34225
  br label %2271

1253:                                             ; preds = %1192
  call void @llvm.lifetime.end.p0(ptr nonnull %77), !noalias !34208
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %79, ptr noundef nonnull align 8 dereferenceable(24) %78, i64 24, i1 false), !noalias !34208
  call void @llvm.lifetime.end.p0(ptr nonnull %78)
  call void @llvm.lifetime.start.p0(ptr nonnull %75), !noalias !34208
  %1254 = getelementptr inbounds nuw i8, ptr %107, i64 8
  %1255 = load ptr, ptr %1254, align 8, !alias.scope !34211, !noalias !34212, !nonnull !1708, !noundef !1708
  %1256 = load i64, ptr %107, align 8, !range !1817, !alias.scope !34211, !noalias !34212, !noundef !1708
  %1257 = getelementptr inbounds nuw [40 x i8], ptr %1255, i64 %1190
  store ptr %1255, ptr %75, align 8, !noalias !34208
  %1258 = getelementptr inbounds nuw i8, ptr %75, i64 16
  store i64 %1256, ptr %1258, align 8, !noalias !34208
  %1259 = getelementptr inbounds nuw i8, ptr %75, i64 8
  store ptr %1255, ptr %1259, align 8, !noalias !34208
  %1260 = getelementptr inbounds nuw i8, ptr %75, i64 24
  store ptr %1257, ptr %1260, align 8, !noalias !34208
  %1261 = load ptr, ptr %555, align 8, !alias.scope !34229, !noalias !34230, !noundef !1708
  %1262 = icmp eq ptr %1261, null
  br i1 %1262, label %1266, label %1263

1263:                                             ; preds = %1253
  %1264 = atomicrmw add ptr %1261, i64 1 monotonic, align 8, !noalias !34230
  %1265 = icmp slt i64 %1264, 0
  br i1 %1265, label %1380, label %1361

1266:                                             ; preds = %1253
  call void @llvm.lifetime.start.p0(ptr nonnull %74), !noalias !34208
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %74, ptr noundef nonnull align 8 dereferenceable(32) %75, i64 32, i1 false), !noalias !34208
  %1267 = getelementptr inbounds nuw i8, ptr %74, i64 24
  %1268 = load ptr, ptr %1267, align 8, !alias.scope !34231, !noalias !34234, !nonnull !1708, !noundef !1708
  %1269 = getelementptr inbounds nuw i8, ptr %74, i64 8
  %1270 = load ptr, ptr %1269, align 8, !alias.scope !34231, !noalias !34234
  %1271 = icmp eq ptr %1270, %1268
  br i1 %1271, label %.loopexit240, label %1272

1272:                                             ; preds = %1266
  %1273 = getelementptr inbounds nuw i8, ptr %79, i64 16
  %1274 = getelementptr inbounds nuw i8, ptr %79, i64 8
  br label %1276

1275:                                             ; preds = %1349, %1346
; call core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(32) %74) #89, !noalias !34230
  br label %2328

1276:                                             ; preds = %1352, %1272
  %1277 = phi ptr [ %1270, %1272 ], [ %1278, %1352 ]
  call void @llvm.experimental.noalias.scope.decl(metadata !34231)
  %1278 = getelementptr inbounds nuw i8, ptr %1277, i64 40
  %1279 = load i64, ptr %1277, align 8, !noalias !34236
  %1280 = getelementptr inbounds nuw i8, ptr %1277, i64 8
  %1281 = load ptr, ptr %1280, align 8, !noalias !34236
  %1282 = icmp eq i64 %1279, 0
  br i1 %1282, label %.loopexit240, label %1340

.loopexit240:                                     ; preds = %1352, %1276, %1266
  %1283 = phi ptr [ %1270, %1266 ], [ %1278, %1276 ], [ %1278, %1352 ]
  call void @llvm.experimental.noalias.scope.decl(metadata !34237)
  call void @llvm.experimental.noalias.scope.decl(metadata !34240)
  %1284 = ptrtoint ptr %1268 to i64
  %1285 = ptrtoint ptr %1283 to i64
  %1286 = sub nuw i64 %1284, %1285
  %1287 = udiv exact i64 %1286, 40
  call void @llvm.experimental.noalias.scope.decl(metadata !34243)
  %1288 = icmp eq ptr %1268, %1283
  br i1 %1288, label %.loopexit239, label %.preheader238

.preheader238:                                    ; preds = %.loopexit240
  %1289 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %1290 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  br label %1291

1291:                                             ; preds = %.preheader238, %1329
  %1292 = phi i64 [ %1294, %1329 ], [ 0, %.preheader238 ]
  %1293 = getelementptr inbounds nuw [40 x i8], ptr %1283, i64 %1292
  %1294 = add nuw nsw i64 %1292, 1
  %1295 = load i64, ptr %1293, align 8, !range !1940, !alias.scope !34246, !noalias !34249, !noundef !1708
  %1296 = icmp ugt i64 %1295, 5
  br i1 %1296, label %1297, label %1329

1297:                                             ; preds = %1291
  %1298 = getelementptr i8, ptr %1293, i64 8
  %1299 = load ptr, ptr %1298, align 8, !alias.scope !34243, !noalias !34249, !nonnull !1708, !noundef !1708
  %1300 = shl i64 %1295, 3
  %1301 = add i64 %1300, -8
  %1302 = load i64, ptr %1289, align 8, !noalias !34250, !noundef !1708
  %1303 = call i64 @llvm.umin.i64(i64 %1301, i64 9223372036854775807)
  %1304 = call i64 @llvm.ssub.sat.i64(i64 %1302, i64 %1303)
  store i64 %1304, ptr %1289, align 8, !noalias !34250
  %1305 = load i64, ptr %1290, align 8, !noalias !34250, !noundef !1708
  %1306 = icmp slt i64 %1304, %1305
  br i1 %1306, label %1307, label %.preheader1947

1307:                                             ; preds = %1297
  store i64 %1304, ptr %1290, align 8, !noalias !34250
  br label %.preheader1947

.preheader1947:                                   ; preds = %1307, %1297
  br label %1308

1308:                                             ; preds = %.preheader1947, %1311
  %1309 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952) acquire, align 8, !noalias !34250
  %1310 = icmp slt i64 %1309, 0
  br i1 %1310, label %1311, label %__rustc::__rust_dealloc (.exit218)

1311:                                             ; preds = %1308
  %1312 = add nsw i64 %1309, 1
  %1313 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 %1309, i64 %1312 acq_rel acquire, align 8, !noalias !34250
  %1314 = extractvalue { i64, i1 } %1313, 1
  br i1 %1314, label %1315, label %1308

1315:                                             ; preds = %1311
  %1316 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1303 monotonic, align 8, !noalias !34250
  %1317 = call i64 @llvm.ssub.sat.i64(i64 %1316, i64 %1303)
  %1318 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !34250
  br label %1319

1319:                                             ; preds = %1322, %1315
  %1320 = phi i64 [ %1318, %1315 ], [ %1325, %1322 ]
  %1321 = icmp slt i64 %1317, %1320
  br i1 %1321, label %1322, label %1326

1322:                                             ; preds = %1319
  %1323 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1320, i64 %1317 monotonic monotonic, align 8, !noalias !34250
  %1324 = extractvalue { i64, i1 } %1323, 1
  %1325 = extractvalue { i64, i1 } %1323, 0
  br i1 %1324, label %1326, label %1319

1326:                                             ; preds = %1322, %1319
  %1327 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 1 release, align 8, !noalias !34250
  br label %__rustc::__rust_dealloc (.exit218)

__rustc::__rust_dealloc (.exit218): ; preds = %1308, %1326
  %1328 = icmp ne i64 %1301, 0
  call void @llvm.assume(i1 %1328), !noalias !34250
  call void @free(ptr noundef nonnull %1299) #88, !noalias !34250
  br label %1329

1329:                                             ; preds = %__rustc::__rust_dealloc (.exit218), %1291
  %1330 = icmp eq i64 %1294, %1287
  br i1 %1330, label %.loopexit239, label %1291

.loopexit239:                                     ; preds = %1329, %.loopexit240
  %1331 = getelementptr inbounds nuw i8, ptr %74, i64 16
  %1332 = load i64, ptr %1331, align 8, !alias.scope !34253, !noalias !34208, !noundef !1708
  %1333 = icmp eq i64 %1332, 0
  br i1 %1333, label %1339, label %1334

1334:                                             ; preds = %.loopexit239
  %1335 = load ptr, ptr %74, align 8, !alias.scope !34253, !noalias !34208, !nonnull !1708, !noundef !1708
  %1336 = mul nuw i64 %1332, 40
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %1335, i64 noundef %1336, i64 noundef range(i64 1, -9223372036854775807) 8) #88, !noalias !34249
  br label %1339

1337:                                             ; preds = %2208, %1509
  %1338 = landingpad { ptr, i32 }
          cleanup
; call core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(32) %75) #89, !noalias !34230
  br label %2988

1339:                                             ; preds = %1334, %.loopexit239
  call void @llvm.lifetime.end.p0(ptr nonnull %74), !noalias !34208
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %85, ptr noundef nonnull align 8 dereferenceable(24) %79, i64 24, i1 false), !noalias !34214
  call void @llvm.lifetime.end.p0(ptr nonnull %75), !noalias !34208
  call void @llvm.lifetime.end.p0(ptr nonnull %79), !noalias !34208
  br label %2271

1340:                                             ; preds = %1276
  %1341 = getelementptr inbounds nuw i8, ptr %1277, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %73, ptr noundef nonnull align 8 dereferenceable(24) %1341, i64 24, i1 false), !noalias !34230
  call void @llvm.experimental.noalias.scope.decl(metadata !34254)
  %1342 = load i64, ptr %1273, align 8, !alias.scope !34254, !noalias !34257, !noundef !1708
  %1343 = load i64, ptr %79, align 8, !range !1817, !alias.scope !34254, !noalias !34257, !noundef !1708
  %1344 = icmp eq i64 %1342, %1343
  br i1 %1344, label %1345, label %1352

1345:                                             ; preds = %1340
; invoke <alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one
  invoke void @<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %79)
          to label %1352 unwind label %1346, !noalias !34259

1346:                                             ; preds = %1345
  %1347 = landingpad { ptr, i32 }
          cleanup
  store ptr %1278, ptr %1269, align 8, !noalias !34208
  %1348 = icmp ugt i64 %1279, 5
  br i1 %1348, label %1349, label %1275

1349:                                             ; preds = %1346
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %1281) ]
  %1350 = shl i64 %1279, 3
  %1351 = add i64 %1350, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %1281, i64 noundef %1351, i64 noundef range(i64 1, -9223372036854775807) 4) #88, !noalias !34260
  br label %1275

1352:                                             ; preds = %1345, %1340
  %1353 = load ptr, ptr %1274, align 8, !alias.scope !34254, !noalias !34257, !nonnull !1708, !noundef !1708
  %1354 = getelementptr inbounds nuw [40 x i8], ptr %1353, i64 %1342
  store i64 %1279, ptr %1354, align 8, !noalias !34263
  %1355 = getelementptr inbounds nuw i8, ptr %1354, i64 8
  store ptr %1281, ptr %1355, align 8, !noalias !34263
  %1356 = getelementptr inbounds nuw i8, ptr %1354, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %1356, ptr noundef nonnull align 8 dereferenceable(24) %73, i64 24, i1 false), !noalias !34263
  %1357 = add i64 %1342, 1
  store i64 %1357, ptr %1273, align 8, !alias.scope !34254, !noalias !34257
  %1358 = icmp eq ptr %1278, %1268
  br i1 %1358, label %.loopexit240, label %1276

1359:                                             ; preds = %2331, %1616, %1474, %1455
  %1360 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #86, !noalias !34230
  unreachable

1361:                                             ; preds = %1263
  %1362 = load ptr, ptr %555, align 8, !alias.scope !34229, !noalias !34230, !nonnull !1708, !noundef !1708
  call void @llvm.lifetime.start.p0(ptr nonnull %72), !noalias !34208
  store ptr %1362, ptr %72, align 8, !noalias !34208
  %1363 = getelementptr inbounds nuw i8, ptr %1362, i64 16
  %1364 = load i64, ptr %1363, align 8, !noalias !34230
  %1365 = icmp eq i64 %1364, -1
  %1366 = getelementptr inbounds nuw i8, ptr %1362, i64 40
  %1367 = load i64, ptr %1366, align 8, !noalias !34230
  %1368 = icmp ne i64 %1367, -1
  %1369 = and i1 %1160, %1368
  br i1 %1369, label %iter.check1835, label %1381

iter.check1835:                                   ; preds = %1361
  %min.iters.check1798 = icmp ult i64 %1135, 8
  br i1 %min.iters.check1798, label %.preheader270.preheader, label %vector.main.loop.iter.check1799

vector.main.loop.iter.check1799:                  ; preds = %iter.check1835
  %min.iters.check1800 = icmp ult i64 %1135, 32
  br i1 %min.iters.check1800, label %vec.epilog.ph1839, label %vector.ph1801

vector.ph1801:                                    ; preds = %vector.main.loop.iter.check1799
  %n.mod.vf1802 = and i64 %1135, 24
  %n.vec1803 = and i64 %1135, -32
  br label %vector.body1804

vector.body1804:                                  ; preds = %vector.body1804, %vector.ph1801
  %index1805 = phi i64 [ 0, %vector.ph1801 ], [ %index.next1826, %vector.body1804 ]
  %vec.ind1806 = phi <8 x i64> [ <i64 0, i64 1, i64 2, i64 3, i64 4, i64 5, i64 6, i64 7>, %vector.ph1801 ], [ %vec.ind.next1827, %vector.body1804 ]
  %vec.phi1807 = phi <8 x i64> [ zeroinitializer, %vector.ph1801 ], [ %1370, %vector.body1804 ]
  %vec.phi1808 = phi <8 x i64> [ zeroinitializer, %vector.ph1801 ], [ %1371, %vector.body1804 ]
  %vec.phi1809 = phi <8 x i64> [ zeroinitializer, %vector.ph1801 ], [ %1372, %vector.body1804 ]
  %vec.phi1810 = phi <8 x i64> [ zeroinitializer, %vector.ph1801 ], [ %1373, %vector.body1804 ]
  %step.add1811 = add nuw <8 x i64> %vec.ind1806, splat (i64 8)
  %step.add.21812 = add nuw <8 x i64> %vec.ind1806, splat (i64 16)
  %step.add.31813 = add nuw <8 x i64> %vec.ind1806, splat (i64 24)
  %wide.gep1814 = getelementptr inbounds nuw [160 x i8], ptr %1136, <8 x i64> %vec.ind1806
  %wide.gep1815 = getelementptr inbounds nuw [160 x i8], ptr %1136, <8 x i64> %step.add1811
  %wide.gep1816 = getelementptr inbounds nuw [160 x i8], ptr %1136, <8 x i64> %step.add.21812
  %wide.gep1817 = getelementptr inbounds nuw [160 x i8], ptr %1136, <8 x i64> %step.add.31813
  %wide.gep1818 = getelementptr i8, <8 x ptr> %wide.gep1814, i64 16
  %wide.gep1819 = getelementptr i8, <8 x ptr> %wide.gep1815, i64 16
  %wide.gep1820 = getelementptr i8, <8 x ptr> %wide.gep1816, i64 16
  %wide.gep1821 = getelementptr i8, <8 x ptr> %wide.gep1817, i64 16
  %wide.masked.gather1822 = call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep1818, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !34230
  %wide.masked.gather1823 = call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep1819, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !34230
  %wide.masked.gather1824 = call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep1820, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !34230
  %wide.masked.gather1825 = call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep1821, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !34230
  %1370 = add <8 x i64> %wide.masked.gather1822, %vec.phi1807
  %1371 = add <8 x i64> %wide.masked.gather1823, %vec.phi1808
  %1372 = add <8 x i64> %wide.masked.gather1824, %vec.phi1809
  %1373 = add <8 x i64> %wide.masked.gather1825, %vec.phi1810
  %index.next1826 = add nuw i64 %index1805, 32
  %vec.ind.next1827 = add nuw <8 x i64> %vec.ind1806, splat (i64 32)
  %1374 = icmp eq i64 %index.next1826, %n.vec1803
  br i1 %1374, label %middle.block1828, label %vector.body1804, !llvm.loop !34264

middle.block1828:                                 ; preds = %vector.body1804
  %bin.rdx1829 = add <8 x i64> %1371, %1370
  %bin.rdx1830 = add <8 x i64> %1372, %bin.rdx1829
  %bin.rdx1831 = add <8 x i64> %1373, %bin.rdx1830
  %1375 = call i64 @llvm.vector.reduce.add.v8i64(<8 x i64> %bin.rdx1831)
  %cmp.n1832 = icmp eq i64 %1135, %n.vec1803
  br i1 %cmp.n1832, label %.loopexit1912, label %vec.epilog.iter.check1837

vec.epilog.iter.check1837:                        ; preds = %middle.block1828
  %min.epilog.iters.check1838 = icmp eq i64 %n.mod.vf1802, 0
  br i1 %min.epilog.iters.check1838, label %.preheader270.preheader, label %vec.epilog.ph1839, !prof !29315

vec.epilog.ph1839:                                ; preds = %vector.main.loop.iter.check1799, %vec.epilog.iter.check1837
  %vec.epilog.resume.val1833 = phi i64 [ %n.vec1803, %vec.epilog.iter.check1837 ], [ 0, %vector.main.loop.iter.check1799 ]
  %bc.merge.rdx1834 = phi i64 [ %1375, %vec.epilog.iter.check1837 ], [ 0, %vector.main.loop.iter.check1799 ]
  %n.vec1841 = and i64 %1135, -8
  %1376 = insertelement <8 x i64> <i64 poison, i64 0, i64 0, i64 0, i64 0, i64 0, i64 0, i64 0>, i64 %bc.merge.rdx1834, i64 0
  %broadcast.splatinsert1842 = insertelement <8 x i64> poison, i64 %vec.epilog.resume.val1833, i64 0
  %broadcast.splat1843 = shufflevector <8 x i64> %broadcast.splatinsert1842, <8 x i64> poison, <8 x i32> zeroinitializer
  %induction1844 = or disjoint <8 x i64> %broadcast.splat1843, <i64 0, i64 1, i64 2, i64 3, i64 4, i64 5, i64 6, i64 7>
  br label %vec.epilog.vector.body1845

vec.epilog.vector.body1845:                       ; preds = %vec.epilog.vector.body1845, %vec.epilog.ph1839
  %index1846 = phi i64 [ %vec.epilog.resume.val1833, %vec.epilog.ph1839 ], [ %index.next1852, %vec.epilog.vector.body1845 ]
  %vec.ind1847 = phi <8 x i64> [ %induction1844, %vec.epilog.ph1839 ], [ %vec.ind.next1853, %vec.epilog.vector.body1845 ]
  %vec.phi1848 = phi <8 x i64> [ %1376, %vec.epilog.ph1839 ], [ %1377, %vec.epilog.vector.body1845 ]
  %wide.gep1849 = getelementptr inbounds nuw [160 x i8], ptr %1136, <8 x i64> %vec.ind1847
  %wide.gep1850 = getelementptr i8, <8 x ptr> %wide.gep1849, i64 16
  %wide.masked.gather1851 = call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep1850, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !34230
  %1377 = add <8 x i64> %wide.masked.gather1851, %vec.phi1848
  %index.next1852 = add nuw i64 %index1846, 8
  %vec.ind.next1853 = add nuw <8 x i64> %vec.ind1847, splat (i64 8)
  %1378 = icmp eq i64 %index.next1852, %n.vec1841
  br i1 %1378, label %vec.epilog.middle.block1854, label %vec.epilog.vector.body1845, !llvm.loop !34265

vec.epilog.middle.block1854:                      ; preds = %vec.epilog.vector.body1845
  %1379 = call i64 @llvm.vector.reduce.add.v8i64(<8 x i64> %1377)
  %cmp.n1855 = icmp eq i64 %1135, %n.vec1841
  br i1 %cmp.n1855, label %.loopexit1912, label %.preheader270.preheader

.preheader270.preheader:                          ; preds = %iter.check1835, %vec.epilog.iter.check1837, %vec.epilog.middle.block1854
  %.ph2186 = phi i64 [ 0, %iter.check1835 ], [ %n.vec1803, %vec.epilog.iter.check1837 ], [ %n.vec1841, %vec.epilog.middle.block1854 ]
  %.ph2187 = phi i64 [ 0, %iter.check1835 ], [ %1375, %vec.epilog.iter.check1837 ], [ %1379, %vec.epilog.middle.block1854 ]
  br label %.preheader270

1380:                                             ; preds = %1263
  call void @llvm.trap()
  unreachable

1381:                                             ; preds = %1471, %1467, %1361
  %1382 = icmp ult i64 %1135, 57646075230342349
  call void @llvm.assume(i1 %1382)
  %1383 = mul nuw nsw i64 %1135, 160
  %1384 = getelementptr inbounds nuw i8, ptr %1136, i64 %1383
  call void @llvm.lifetime.start.p0(ptr nonnull %71), !noalias !34208
  store ptr %1136, ptr %71, align 8, !noalias !34208
  %1385 = getelementptr inbounds nuw i8, ptr %71, i64 8
  store ptr %1136, ptr %1385, align 8, !noalias !34208
  %1386 = getelementptr inbounds nuw i8, ptr %71, i64 16
  store i64 %1156, ptr %1386, align 8, !noalias !34208
  %1387 = getelementptr inbounds nuw i8, ptr %71, i64 24
  store ptr %1384, ptr %1387, align 8, !noalias !34208
  %1388 = icmp eq i64 %1135, 0
  br i1 %1388, label %.loopexit268, label %1389

1389:                                             ; preds = %1381
  %1390 = getelementptr inbounds nuw i8, ptr %70, i64 8
  %1391 = getelementptr inbounds nuw i8, ptr %70, i64 24
  %1392 = getelementptr inbounds nuw i8, ptr %70, i64 32
  %1393 = getelementptr inbounds nuw i8, ptr %70, i64 40
  %1394 = getelementptr inbounds nuw i8, ptr %70, i64 16
  %1395 = getelementptr inbounds nuw i8, ptr %69, i64 16
  %1396 = getelementptr inbounds nuw i8, ptr %69, i64 8
  %1397 = getelementptr inbounds nuw i8, ptr %69, i64 24
  %1398 = getelementptr inbounds nuw i8, ptr %70, i64 96
  %1399 = getelementptr inbounds nuw i8, ptr %70, i64 48
  %1400 = getelementptr inbounds nuw i8, ptr %70, i64 56
  %1401 = getelementptr inbounds nuw i8, ptr %70, i64 64
  %1402 = getelementptr inbounds nuw i8, ptr %1362, i64 80
  %1403 = getelementptr inbounds nuw i8, ptr %55, i64 1
  %1404 = getelementptr inbounds nuw i8, ptr %55, i64 8
  %1405 = getelementptr inbounds nuw i8, ptr %55, i64 16
  %1406 = getelementptr inbounds nuw i8, ptr %4, i64 632
  %1407 = getelementptr inbounds nuw i8, ptr %4, i64 1228
  %1408 = zext nneg i8 %1183 to i64
  %1409 = getelementptr inbounds nuw i8, ptr %1362, i64 296
  %1410 = getelementptr inbounds nuw i8, ptr %1362, i64 272
  %1411 = getelementptr inbounds nuw i8, ptr %4, i64 1048
  %1412 = getelementptr inbounds nuw i8, ptr %4, i64 1056
  %1413 = getelementptr inbounds nuw i8, ptr %1362, i64 104
  %1414 = getelementptr inbounds nuw i8, ptr %49, i64 1
  %1415 = getelementptr inbounds nuw i8, ptr %49, i64 8
  %1416 = getelementptr inbounds nuw i8, ptr %49, i64 16
  %1417 = getelementptr inbounds nuw i8, ptr %54, i64 8
  %1418 = getelementptr inbounds nuw i8, ptr %4, i64 888
  %1419 = getelementptr inbounds nuw i8, ptr %43, i64 1
  %1420 = getelementptr inbounds nuw i8, ptr %43, i64 8
  %1421 = getelementptr inbounds nuw i8, ptr %43, i64 16
  %1422 = getelementptr inbounds nuw i8, ptr %50, i64 8
  %1423 = getelementptr inbounds nuw i8, ptr %52, i64 1
  %1424 = getelementptr inbounds nuw i8, ptr %52, i64 8
  %1425 = getelementptr inbounds nuw i8, ptr %52, i64 16
  %1426 = getelementptr inbounds nuw i8, ptr %51, i64 8
  %1427 = getelementptr inbounds nuw i8, ptr %48, i64 8
  %1428 = getelementptr inbounds nuw i8, ptr %47, i64 8
  %1429 = getelementptr inbounds nuw i8, ptr %45, i64 1
  %1430 = getelementptr inbounds nuw i8, ptr %45, i64 8
  %1431 = getelementptr inbounds nuw i8, ptr %45, i64 16
  %1432 = getelementptr inbounds nuw i8, ptr %44, i64 8
  %1433 = getelementptr inbounds nuw i8, ptr %79, i64 16
  %1434 = getelementptr inbounds nuw i8, ptr %79, i64 8
  %1435 = getelementptr inbounds nuw i8, ptr %70, i64 88
  %1436 = getelementptr inbounds nuw i8, ptr %70, i64 120
  %1437 = getelementptr inbounds nuw i8, ptr %40, i64 1
  %1438 = getelementptr inbounds nuw i8, ptr %40, i64 8
  %1439 = getelementptr inbounds nuw i8, ptr %40, i64 16
  br label %1475

.preheader270:                                    ; preds = %.preheader270.preheader, %.preheader270
  %1440 = phi i64 [ %1447, %.preheader270 ], [ %.ph2186, %.preheader270.preheader ]
  %1441 = phi i64 [ %1446, %.preheader270 ], [ %.ph2187, %.preheader270.preheader ]
  %1442 = getelementptr inbounds nuw [160 x i8], ptr %1136, i64 %1440
  %1443 = getelementptr i8, ptr %1442, i64 16
  %1444 = load i64, ptr %1443, align 8, !noalias !34230, !noundef !1708
  %1445 = icmp ult i64 %1444, 104811045873349726
  call void @llvm.assume(i1 %1445)
  %1446 = add i64 %1444, %1441
  %1447 = add nuw i64 %1440, 1
  %1448 = icmp eq i64 %1447, %1135
  br i1 %1448, label %.loopexit1912, label %.preheader270, !llvm.loop !34266

1449:                                             ; preds = %1474, %1456
  %1450 = phi i1 [ %1457, %1456 ], [ %1619, %1474 ]
  %1451 = phi i1 [ %1458, %1456 ], [ false, %1474 ]
  %1452 = phi { ptr, i32 } [ %1459, %1456 ], [ %1620, %1474 ]
  %1453 = atomicrmw sub ptr %1362, i64 1 release, align 8, !noalias !34267
  %1454 = icmp eq i64 %1453, 1
  br i1 %1454, label %1455, label %2268

1455:                                             ; preds = %1449
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %72) #87
          to label %2268 unwind label %1359, !noalias !34230

1456:                                             ; preds = %2204, %1510, %.loopexit268, %1471, %1466
  %1457 = phi i1 [ false, %2204 ], [ true, %1510 ], [ true, %.loopexit268 ], [ true, %1471 ], [ true, %1466 ]
  %1458 = phi i1 [ false, %2204 ], [ false, %1510 ], [ false, %.loopexit268 ], [ true, %1471 ], [ true, %1466 ]
  %1459 = landingpad { ptr, i32 }
          cleanup
  br label %1449

.loopexit1912:                                    ; preds = %.preheader270, %vec.epilog.middle.block1854, %middle.block1828
  %.lcssa1721 = phi i64 [ %1379, %vec.epilog.middle.block1854 ], [ %1375, %middle.block1828 ], [ %1446, %.preheader270 ]
  %1460 = getelementptr inbounds nuw i8, ptr %4, i64 912
  %1461 = getelementptr inbounds nuw i8, ptr %4, i64 928
  %1462 = load i64, ptr %1461, align 16, !alias.scope !34272, !noalias !34230, !noundef !1708
  %1463 = load i64, ptr %1460, align 16, !range !1817, !alias.scope !34272, !noalias !34230, !noundef !1708
  %1464 = sub i64 %1463, %1462
  %1465 = icmp ugt i64 %.lcssa1721, %1464
  br i1 %1465, label %1466, label %1467, !prof !34277

1466:                                             ; preds = %.loopexit1912
; invoke <alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global>
  invoke fastcc void @<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.13412714042204560522)(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %1460, i64 noundef %1462, i64 noundef %.lcssa1721, i64 noundef 8, i64 noundef 80)
          to label %1467 unwind label %1456, !noalias !34230

1467:                                             ; preds = %1466, %.loopexit1912
  %1468 = getelementptr inbounds nuw i8, ptr %4, i64 1016
  %1469 = load i64, ptr %1468, align 8, !alias.scope !34278, !noalias !34230, !noundef !1708
  %1470 = icmp ugt i64 %.lcssa1721, %1469
  br i1 %1470, label %1471, label %1381, !prof !34277

1471:                                             ; preds = %1467
  %1472 = getelementptr inbounds nuw i8, ptr %4, i64 1000
; invoke <hashbrown::raw::RawTable<usize>>::reserve_rehash::<<purrdf_sparql_eval::scratch::ScratchInterner>::reserve_ghosts::{closure#0}>
  %1473 = invoke { i64, i64 } @<hashbrown::raw::RawTable<usize>>::reserve_rehash::<<purrdf_sparql_eval::scratch::ScratchInterner>::reserve_ghosts::{closure#0}>(ptr noalias nofree noundef nonnull align 8 dereferenceable(32) %1472, i64 noundef %.lcssa1721, ptr noundef nonnull align 8 %1460, i1 noundef zeroext true) #87
          to label %1381 unwind label %1456, !noalias !34230

1474:                                             ; preds = %2267, %2264, %2261
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>(ptr noalias nofree noundef align 8 dereferenceable(32) %71) #89
          to label %1449 unwind label %1359, !noalias !34230

1475:                                             ; preds = %1653, %1389
  %1476 = phi ptr [ %1255, %1389 ], [ %1612, %1653 ]
  %1477 = phi ptr [ %1136, %1389 ], [ %1478, %1653 ]
  call void @llvm.experimental.noalias.scope.decl(metadata !34281)
  %1478 = getelementptr inbounds nuw i8, ptr %1477, i64 160
  store ptr %1478, ptr %1385, align 8, !alias.scope !34281, !noalias !34284
  %1479 = load i64, ptr %1477, align 8, !noalias !34286
  %1480 = icmp eq i64 %1479, -1
  br i1 %1480, label %.loopexit268, label %1481

1481:                                             ; preds = %1475
  %1482 = getelementptr inbounds nuw i8, ptr %1477, i64 8
  call void @llvm.lifetime.start.p0(ptr nonnull %70), !noalias !34208
  store i64 %1479, ptr %70, align 8, !noalias !34208
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(152) %1390, ptr noundef nonnull align 8 dereferenceable(152) %1482, i64 152, i1 false), !noalias !34230
  %1483 = load i64, ptr %1391, align 8, !noalias !34208
  %1484 = load ptr, ptr %1392, align 8, !noalias !34208
  %1485 = load i64, ptr %1393, align 8, !noalias !34208
  call void @llvm.lifetime.start.p0(ptr nonnull %69), !noalias !34208
  %1486 = load ptr, ptr %1390, align 8, !noalias !34208, !nonnull !1708, !noundef !1708
  %1487 = load i64, ptr %1394, align 8, !noalias !34208, !noundef !1708
  %1488 = icmp ult i64 %1487, 104811045873349726
  call void @llvm.assume(i1 %1488)
  %1489 = getelementptr inbounds nuw [88 x i8], ptr %1486, i64 %1487
  store ptr %1486, ptr %69, align 8, !noalias !34208
  store i64 %1479, ptr %1395, align 8, !noalias !34208
  store ptr %1486, ptr %1396, align 8, !noalias !34208
  store ptr %1489, ptr %1397, align 8, !noalias !34208
  %1490 = load ptr, ptr %1400, align 8, !noalias !34208, !nonnull !1708, !noundef !1708
  %1491 = load i64, ptr %1399, align 8, !range !1817, !noalias !34208, !noundef !1708
  %1492 = load i64, ptr %1401, align 8, !noalias !34208, !noundef !1708
  %1493 = icmp ult i64 %1492, 288230376151711744
  call void @llvm.assume(i1 %1493)
  %1494 = shl nuw nsw i64 %1492, 5
  %1495 = getelementptr inbounds nuw i8, ptr %1490, i64 %1494
  %1496 = icmp eq i64 %1492, 0
  br i1 %1496, label %.loopexit267, label %1497

1497:                                             ; preds = %1481
  %1498 = load i64, ptr %1398, align 8, !noalias !34208, !noundef !1708
  %1499 = icmp ult i64 %1485, 384307168202282326
  %1500 = ptrtoint ptr %1489 to i64
  br label %1595

.loopexit268:                                     ; preds = %1653, %1475, %1381
  %1501 = phi ptr [ %1255, %1381 ], [ %1612, %1653 ], [ %1476, %1475 ]
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>(ptr noalias nofree noundef align 8 dereferenceable(32) %71)
          to label %1502 unwind label %1456, !noalias !34230

1502:                                             ; preds = %.loopexit268
  call void @llvm.lifetime.end.p0(ptr nonnull %71), !noalias !34208
  %1503 = xor i1 %1159, true
  %1504 = or i1 %1181, %1503
  %1505 = select i1 %1504, i1 true, i1 %1365
  br i1 %1505, label %1506, label %1510

1506:                                             ; preds = %1512, %1502
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %85, ptr noundef nonnull align 8 dereferenceable(24) %79, i64 24, i1 false), !noalias !34214
  %1507 = atomicrmw sub ptr %1362, i64 1 release, align 8, !noalias !34287
  %1508 = icmp eq i64 %1507, 1
  br i1 %1508, label %1509, label %1513

1509:                                             ; preds = %1506
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %72) #87
          to label %1513 unwind label %1337, !noalias !34230

1510:                                             ; preds = %1502
  call void @llvm.lifetime.start.p0(ptr nonnull %61), !noalias !34208
  call void @llvm.lifetime.start.p0(ptr nonnull %60), !noalias !34208
  store i64 1, ptr %60, align 8, !noalias !34208
  %1511 = getelementptr inbounds nuw i8, ptr %60, i64 8
  store i64 0, ptr %1511, align 8, !noalias !34208
; invoke <purrdf_sparql_eval::governor::GovernorState>::commit_reported_items
  invoke void @<purrdf_sparql_eval::governor::GovernorState>::commit_reported_items(ptr noalias nofree noundef nonnull sret([40 x i8]) align 8 captures(address) dereferenceable(40) %61, ptr noundef nonnull align 8 %1363, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) %60, i64 noundef 1)
          to label %1512 unwind label %1456, !noalias !34230

1512:                                             ; preds = %1510
  call void @llvm.lifetime.end.p0(ptr nonnull %60), !noalias !34208
  call void @llvm.lifetime.end.p0(ptr nonnull %61), !noalias !34208
  br label %1506

1513:                                             ; preds = %1509, %1506
  call void @llvm.lifetime.end.p0(ptr nonnull %72), !noalias !34208
  call void @llvm.experimental.noalias.scope.decl(metadata !34292)
  call void @llvm.experimental.noalias.scope.decl(metadata !34295), !noalias !34298
  %1514 = load ptr, ptr %1260, align 8, !alias.scope !34299, !noalias !34300, !nonnull !1708, !noundef !1708
  %1515 = ptrtoint ptr %1514 to i64
  %1516 = ptrtoint ptr %1501 to i64
  %1517 = sub nuw i64 %1515, %1516
  %1518 = udiv exact i64 %1517, 40
  call void @llvm.experimental.noalias.scope.decl(metadata !34301), !noalias !34298
  %1519 = icmp eq ptr %1514, %1501
  br i1 %1519, label %.loopexit242, label %.preheader241

.preheader241:                                    ; preds = %1513
  %1520 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %1521 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  br label %1522

1522:                                             ; preds = %.preheader241, %1560
  %1523 = phi i64 [ %1525, %1560 ], [ 0, %.preheader241 ]
  %1524 = getelementptr inbounds nuw [40 x i8], ptr %1501, i64 %1523
  %1525 = add nuw nsw i64 %1523, 1
  %1526 = load i64, ptr %1524, align 8, !range !1940, !alias.scope !34304, !noalias !34307, !noundef !1708
  %1527 = icmp ugt i64 %1526, 5
  br i1 %1527, label %1528, label %1560

1528:                                             ; preds = %1522
  %1529 = getelementptr i8, ptr %1524, i64 8
  %1530 = load ptr, ptr %1529, align 8, !alias.scope !34301, !noalias !34307, !nonnull !1708, !noundef !1708
  %1531 = shl i64 %1526, 3
  %1532 = add i64 %1531, -8
  %1533 = load i64, ptr %1520, align 8, !noalias !34308, !noundef !1708
  %1534 = call i64 @llvm.umin.i64(i64 %1532, i64 9223372036854775807)
  %1535 = call i64 @llvm.ssub.sat.i64(i64 %1533, i64 %1534)
  store i64 %1535, ptr %1520, align 8, !noalias !34308
  %1536 = load i64, ptr %1521, align 8, !noalias !34308, !noundef !1708
  %1537 = icmp slt i64 %1535, %1536
  br i1 %1537, label %1538, label %.preheader1954

1538:                                             ; preds = %1528
  store i64 %1535, ptr %1521, align 8, !noalias !34308
  br label %.preheader1954

.preheader1954:                                   ; preds = %1538, %1528
  br label %1539

1539:                                             ; preds = %.preheader1954, %1542
  %1540 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952) acquire, align 8, !noalias !34308
  %1541 = icmp slt i64 %1540, 0
  br i1 %1541, label %1542, label %__rustc::__rust_dealloc (.exit219)

1542:                                             ; preds = %1539
  %1543 = add nsw i64 %1540, 1
  %1544 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 %1540, i64 %1543 acq_rel acquire, align 8, !noalias !34308
  %1545 = extractvalue { i64, i1 } %1544, 1
  br i1 %1545, label %1546, label %1539

1546:                                             ; preds = %1542
  %1547 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1534 monotonic, align 8, !noalias !34308
  %1548 = call i64 @llvm.ssub.sat.i64(i64 %1547, i64 %1534)
  %1549 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !34308
  br label %1550

1550:                                             ; preds = %1553, %1546
  %1551 = phi i64 [ %1549, %1546 ], [ %1556, %1553 ]
  %1552 = icmp slt i64 %1548, %1551
  br i1 %1552, label %1553, label %1557

1553:                                             ; preds = %1550
  %1554 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1551, i64 %1548 monotonic monotonic, align 8, !noalias !34308
  %1555 = extractvalue { i64, i1 } %1554, 1
  %1556 = extractvalue { i64, i1 } %1554, 0
  br i1 %1555, label %1557, label %1550

1557:                                             ; preds = %1553, %1550
  %1558 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 1 release, align 8, !noalias !34308
  br label %__rustc::__rust_dealloc (.exit219)

__rustc::__rust_dealloc (.exit219): ; preds = %1539, %1557
  %1559 = icmp ne i64 %1532, 0
  call void @llvm.assume(i1 %1559), !noalias !34308
  call void @free(ptr noundef nonnull %1530) #88, !noalias !34308
  br label %1560

1560:                                             ; preds = %__rustc::__rust_dealloc (.exit219), %1522
  %1561 = icmp eq i64 %1525, %1518
  br i1 %1561, label %.loopexit242, label %1522

.loopexit242:                                     ; preds = %1560, %1513
  %1562 = load i64, ptr %1258, align 8, !alias.scope !34299, !noalias !34300, !noundef !1708
  %1563 = icmp eq i64 %1562, 0
  br i1 %1563, label %2338, label %2333

1564:                                             ; preds = %1858
  %1565 = landingpad { ptr, i32 }
          cleanup
  store ptr %1854, ptr %1396, align 8, !noalias !34208
  br label %1590

1566:                                             ; preds = %1836
  %1567 = landingpad { ptr, i32 }
          cleanup
  br label %1590

1568:                                             ; preds = %1924
  %1569 = landingpad { ptr, i32 }
          cleanup
  store ptr %1920, ptr %1396, align 8, !noalias !34208
  br label %1590

1570:                                             ; preds = %1818
  %1571 = landingpad { ptr, i32 }
          cleanup
  store ptr %1814, ptr %1396, align 8, !noalias !34208
  br label %1590

1572:                                             ; preds = %2143
  %1573 = landingpad { ptr, i32 }
          cleanup
  store ptr %2139, ptr %1396, align 8, !noalias !34208
  br label %1590

1574:                                             ; preds = %2077
  %1575 = landingpad { ptr, i32 }
          cleanup
  br label %1590

1576:                                             ; preds = %2050
  %1577 = landingpad { ptr, i32 }
          cleanup
  store ptr %2046, ptr %1396, align 8, !noalias !34208
  br label %1590

1578:                                             ; preds = %1992
  %1579 = landingpad { ptr, i32 }
          cleanup
  store ptr %1988, ptr %1396, align 8, !noalias !34208
  br label %1590

1580:                                             ; preds = %1956, %1940, %1908
  %1581 = landingpad { ptr, i32 }
          cleanup
  br label %1590

1582:                                             ; preds = %.preheader262
  %1583 = landingpad { ptr, i32 }
          cleanup
  br label %1590

1584:                                             ; preds = %1674
  %1585 = landingpad { ptr, i32 }
          cleanup
  br label %1590

1586:                                             ; preds = %2122, %2118, %2109
  %1587 = landingpad { ptr, i32 }
          cleanup
  br label %1590

1588:                                             ; preds = %1699, %1657
  %1589 = landingpad { ptr, i32 }
          cleanup
  br label %1590

1590:                                             ; preds = %2170, %2167, %1588, %1586, %1584, %1582, %1580, %1578, %1576, %1574, %1572, %1570, %1568, %1566, %1564
  %1591 = phi { ptr, i32 } [ %2168, %2167 ], [ %2168, %2170 ], [ %1565, %1564 ], [ %1567, %1566 ], [ %1569, %1568 ], [ %1571, %1570 ], [ %1573, %1572 ], [ %1575, %1574 ], [ %1577, %1576 ], [ %1579, %1578 ], [ %1581, %1580 ], [ %1583, %1582 ], [ %1585, %1584 ], [ %1587, %1586 ], [ %1589, %1588 ]
  %1592 = icmp eq i64 %1491, 0
  br i1 %1592, label %1616, label %1593

1593:                                             ; preds = %1590
  %1594 = shl nuw i64 %1491, 5
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %1490, i64 noundef %1594, i64 noundef range(i64 1, -9223372036854775807) 8) #88, !noalias !34311
  br label %1616

1595:                                             ; preds = %.loopexit248, %1497
  %1596 = phi ptr [ %1476, %1497 ], [ %2153, %.loopexit248 ]
  %1597 = phi ptr [ %1486, %1497 ], [ %1972, %.loopexit248 ]
  %1598 = phi i64 [ 0, %1497 ], [ %1697, %.loopexit248 ]
  %1599 = phi ptr [ %1490, %1497 ], [ %1602, %.loopexit248 ]
  %1600 = phi ptr [ %1486, %1497 ], [ %1974, %.loopexit248 ]
  %1601 = phi i64 [ %1498, %1497 ], [ %1973, %.loopexit248 ]
  %1602 = getelementptr inbounds nuw i8, ptr %1599, i64 32
  %1603 = load i64, ptr %1599, align 8, !noalias !34314
  %1604 = getelementptr inbounds nuw i8, ptr %1599, i64 8
  %1605 = load i64, ptr %1604, align 8, !noalias !34314
  %1606 = getelementptr inbounds nuw i8, ptr %1599, i64 16
  %1607 = load i64, ptr %1606, align 8, !noalias !34314
  %1608 = getelementptr inbounds nuw i8, ptr %1599, i64 24
  %1609 = load i64, ptr %1608, align 8, !noalias !34314
  %1610 = icmp eq i64 %1603, 0
  %1611 = select i1 %1610, i1 true, i1 %1365
  br i1 %1611, label %1655, label %1662

.loopexit267:                                     ; preds = %.loopexit248, %1481
  %1612 = phi ptr [ %1476, %1481 ], [ %2153, %.loopexit248 ]
  %1613 = icmp eq i64 %1491, 0
  br i1 %1613, label %1617, label %1614

1614:                                             ; preds = %.loopexit267
  %1615 = shl nuw i64 %1491, 5
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %1490, i64 noundef %1615, i64 noundef range(i64 1, -9223372036854775807) 8) #88, !noalias !34317
  br label %1617

1616:                                             ; preds = %1593, %1590
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>(ptr noalias nofree noundef align 8 dereferenceable(32) %69) #89
          to label %1618 unwind label %1359, !noalias !34230

1617:                                             ; preds = %1614, %.loopexit267
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>(ptr noalias nofree noundef align 8 dereferenceable(32) %69)
          to label %1628 unwind label %1624, !noalias !34230

1618:                                             ; preds = %1626, %1624, %1616
  %1619 = phi i1 [ true, %1616 ], [ true, %1624 ], [ false, %1626 ]
  %1620 = phi { ptr, i32 } [ %1591, %1616 ], [ %1625, %1624 ], [ %1627, %1626 ]
  %1621 = icmp eq i64 %1483, 0
  br i1 %1621, label %1632, label %1622

1622:                                             ; preds = %1618
  %1623 = mul nuw i64 %1483, 24
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %1484) ]
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %1484, i64 noundef %1623, i64 noundef range(i64 1, -9223372036854775807) 8) #88, !noalias !34230
  br label %1632

1624:                                             ; preds = %1617
  %1625 = landingpad { ptr, i32 }
          cleanup
  br label %1618

1626:                                             ; preds = %2185
  %1627 = landingpad { ptr, i32 }
          cleanup
  br label %1618

1628:                                             ; preds = %1617
  call void @llvm.lifetime.end.p0(ptr nonnull %69), !noalias !34208
  %1629 = icmp eq i64 %1483, 0
  br i1 %1629, label %1639, label %1630

1630:                                             ; preds = %1628
  %1631 = mul nuw i64 %1483, 24
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %1484) ]
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %1484, i64 noundef %1631, i64 noundef range(i64 1, -9223372036854775807) 8) #88, !noalias !34230
  br label %1639

1632:                                             ; preds = %1622, %1618
  call void @llvm.experimental.noalias.scope.decl(metadata !34320)
  %1633 = load ptr, ptr %1435, align 8, !alias.scope !34320, !noalias !34208, !noundef !1708
  %1634 = icmp eq ptr %1633, null
  br i1 %1634, label %2261, label %1635

1635:                                             ; preds = %1632
  %1636 = atomicrmw sub ptr %1633, i64 1 release, align 8, !noalias !34323
  %1637 = icmp eq i64 %1636, 1
  br i1 %1637, label %1638, label %2261

1638:                                             ; preds = %1635
  fence acquire
; call <alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %1435) #87, !noalias !34230
  br label %2261

1639:                                             ; preds = %1630, %1628
  call void @llvm.experimental.noalias.scope.decl(metadata !34328)
  %1640 = load ptr, ptr %1435, align 8, !alias.scope !34328, !noalias !34208, !noundef !1708
  %1641 = icmp eq ptr %1640, null
  br i1 %1641, label %1646, label %1642

1642:                                             ; preds = %1639
  %1643 = atomicrmw sub ptr %1640, i64 1 release, align 8, !noalias !34331
  %1644 = icmp eq i64 %1643, 1
  br i1 %1644, label %1645, label %1646

1645:                                             ; preds = %1642
  fence acquire
; call <alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %1435) #87, !noalias !34230
  br label %1646

1646:                                             ; preds = %1645, %1642, %1639
  call void @llvm.experimental.noalias.scope.decl(metadata !34336)
  %1647 = load ptr, ptr %1436, align 8, !alias.scope !34336, !noalias !34208, !noundef !1708
  %1648 = icmp eq ptr %1647, null
  br i1 %1648, label %1653, label %1649

1649:                                             ; preds = %1646
  %1650 = atomicrmw sub ptr %1647, i64 1 release, align 8, !noalias !34339
  %1651 = icmp eq i64 %1650, 1
  br i1 %1651, label %1652, label %1653

1652:                                             ; preds = %1649
  fence acquire
; call <alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %1436) #87, !noalias !34230
  br label %1653

1653:                                             ; preds = %1652, %1649, %1646
  call void @llvm.lifetime.end.p0(ptr nonnull %70), !noalias !34208
  %1654 = icmp eq ptr %1478, %1384
  br i1 %1654, label %.loopexit268, label %1475

1655:                                             ; preds = %1688, %1682, %1678, %1595
  call void @llvm.assume(i1 %1499)
  %1656 = icmp ugt i64 %1598, %1485
  br i1 %1656, label %1657, label %1694, !prof !1803

1657:                                             ; preds = %1655
  call void @llvm.lifetime.start.p0(ptr nonnull %59), !noalias !34208
  store i64 %1598, ptr %59, align 8, !noalias !34208
  call void @llvm.lifetime.start.p0(ptr nonnull %58), !noalias !34208
  store i64 %1485, ptr %58, align 8, !noalias !34208
  call void @llvm.lifetime.start.p0(ptr nonnull %57), !noalias !34208
  store ptr %59, ptr %57, align 8, !noalias !34208
  %1658 = getelementptr inbounds nuw i8, ptr %57, i64 8
  store ptr @<usize as core::fmt::Debug>::fmt, ptr %1658, align 8, !noalias !34208
  %1659 = getelementptr inbounds nuw i8, ptr %57, i64 16
  store ptr %58, ptr %1659, align 8, !noalias !34208
  %1660 = getelementptr inbounds nuw i8, ptr %57, i64 24
  store ptr @<usize as core::fmt::Debug>::fmt, ptr %1660, align 8, !noalias !34208
; invoke core::panicking::panic_fmt
  invoke void @core::panicking::panic_fmt(ptr noundef nonnull @anon.a12f493ba210922c94e5446ac885c35e.2054, ptr noundef nonnull %57, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.a12f493ba210922c94e5446ac885c35e.300) #92
          to label %1661 unwind label %1588, !noalias !34230

1661:                                             ; preds = %1657
  unreachable

1662:                                             ; preds = %1595
  call void @llvm.lifetime.start.p0(ptr nonnull %56), !noalias !34344
  %1663 = load atomic i64, ptr %1402 monotonic, align 8, !noalias !34351
  br label %1664

1664:                                             ; preds = %1664, %1662
  %1665 = phi i64 [ %1663, %1662 ], [ %1669, %1664 ]
  %1666 = call i64 @llvm.uadd.sat.i64(i64 %1665, i64 %1603)
  %1667 = cmpxchg weak ptr %1402, i64 %1665, i64 %1666 monotonic monotonic, align 8, !noalias !34351
  %1668 = extractvalue { i64, i1 } %1667, 1
  %1669 = extractvalue { i64, i1 } %1667, 0
  br i1 %1668, label %1670, label %1664

1670:                                             ; preds = %1664
  %1671 = call i64 @llvm.uadd.sat.i64(i64 %1669, i64 %1603)
  %1672 = load i64, ptr %1363, align 8, !noalias !34351
  %1673 = icmp ugt i64 %1671, %1672
  br i1 %1673, label %1674, label %1678

1674:                                             ; preds = %1670
  call void @llvm.lifetime.start.p0(ptr nonnull %55), !noalias !34354
  store i8 0, ptr %1403, align 1, !noalias !34354
  store i64 %1672, ptr %1404, align 8, !noalias !34354
  store i64 %1671, ptr %1405, align 8, !noalias !34354
  store i8 0, ptr %55, align 8, !noalias !34354
; invoke <purrdf_sparql_eval::governor::GovernorState>::trip
  invoke fastcc void @<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.13412714042204560522)(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(24) %56, ptr noundef nonnull align 8 %1363, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %55)
          to label %1675 unwind label %1584, !noalias !34230

1675:                                             ; preds = %1674
  call void @llvm.lifetime.end.p0(ptr nonnull %55), !noalias !34354
  %1676 = load i8, ptr %56, align 8, !noalias !34344
  %1677 = icmp eq i8 %1676, -1
  br i1 %1677, label %1678, label %1681

1678:                                             ; preds = %1675, %1670
  call void @llvm.lifetime.end.p0(ptr nonnull %56), !noalias !34344
  %1679 = load ptr, ptr %1406, align 8, !alias.scope !34229, !noalias !34230, !noundef !1708
  %1680 = icmp eq ptr %1679, null
  br i1 %1680, label %1655, label %1682

1681:                                             ; preds = %1675
  call void @llvm.lifetime.end.p0(ptr nonnull %56), !noalias !34344
  br label %.loopexit253

1682:                                             ; preds = %1678
  %1683 = load i32, ptr %1407, align 4, !alias.scope !34229, !noalias !34230, !noundef !1708
  %1684 = getelementptr i8, ptr %1679, i64 56
  %1685 = load i64, ptr %1684, align 8, !noalias !34230, !noundef !1708
  %1686 = zext i32 %1683 to i64
  %1687 = icmp ugt i64 %1685, %1686
  br i1 %1687, label %1688, label %1655

1688:                                             ; preds = %1682
  %1689 = getelementptr i8, ptr %1679, i64 48
  %1690 = load ptr, ptr %1689, align 8, !noalias !34230, !nonnull !1708, !noundef !1708
  %1691 = getelementptr inbounds nuw [136 x i8], ptr %1690, i64 %1686
  %1692 = getelementptr inbounds nuw [8 x i8], ptr %1691, i64 %1408
  %1693 = atomicrmw add ptr %1692, i64 %1603 monotonic, align 8, !noalias !34230
  br label %1655

1694:                                             ; preds = %1655
  %1695 = icmp ult i64 %1605, %1598
  %1696 = call i64 @llvm.umin.i64(i64 %1605, i64 range(i64 0, 384307168202282326) %1485)
  %1697 = select i1 %1695, i64 %1598, i64 %1696
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %1484) ]
  %1698 = icmp samesign ult i64 %1697, %1598
  br i1 %1698, label %1699, label %1700, !prof !10448

1699:                                             ; preds = %1694
; invoke core::slice::index::slice_index_fail
  invoke void @core::slice::index::slice_index_fail(i64 noundef %1598, i64 noundef %1697, i64 noundef %1485, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.a12f493ba210922c94e5446ac885c35e.301) #90
          to label %2184 unwind label %1588, !noalias !34230

1700:                                             ; preds = %1694
  %1701 = mul nuw nsw i64 %1598, 24
  %1702 = getelementptr inbounds nuw i8, ptr %1484, i64 %1701
  %1703 = mul nuw nsw i64 %1697, 24
  %1704 = getelementptr inbounds nuw i8, ptr %1484, i64 %1703
  %1705 = icmp eq i64 %1598, %1697
  br i1 %1705, label %.loopexit265, label %1706

1706:                                             ; preds = %1700
  %1707 = sub nuw nsw i64 %1703, %1701
  %1708 = udiv exact i64 %1707, 24
  br label %1709

1709:                                             ; preds = %1724, %1706
  %1710 = phi i64 [ 0, %1706 ], [ %1725, %1724 ]
  %1711 = phi i64 [ 0, %1706 ], [ %1726, %1724 ]
  %1712 = phi i64 [ 0, %1706 ], [ %1727, %1724 ]
  %1713 = phi i64 [ 0, %1706 ], [ %1728, %1724 ]
  %1714 = getelementptr inbounds nuw [24 x i8], ptr %1702, i64 %1713
  %1715 = load i8, ptr %1714, align 8, !range !10848, !noalias !34355, !noundef !1708
  %1716 = getelementptr i8, ptr %1714, i64 8
  %1717 = load i64, ptr %1716, align 8, !noalias !34355
  switch i8 %1715, label %.unreachabledefault [
    i8 0, label %1718
    i8 1, label %1720
    i8 2, label %1724
    i8 3, label %1722
  ]

.unreachabledefault:                              ; preds = %1709
  unreachable

default.unreachable1172:                          ; preds = %.preheader252
  unreachable

1718:                                             ; preds = %1709
  %1719 = call i64 @llvm.uadd.sat.i64(i64 %1712, i64 %1717)
  br label %1724

1720:                                             ; preds = %1709
  %1721 = call i64 @llvm.uadd.sat.i64(i64 %1711, i64 %1717)
  br label %1724

1722:                                             ; preds = %1709
  %1723 = call i64 @llvm.umax.i64(i64 %1710, i64 %1717)
  br label %1724

1724:                                             ; preds = %1722, %1720, %1718, %1709
  %1725 = phi i64 [ %1710, %1718 ], [ %1710, %1720 ], [ %1723, %1722 ], [ %1710, %1709 ]
  %1726 = phi i64 [ %1711, %1718 ], [ %1721, %1720 ], [ %1711, %1722 ], [ %1711, %1709 ]
  %1727 = phi i64 [ %1719, %1718 ], [ %1712, %1720 ], [ %1712, %1722 ], [ %1712, %1709 ]
  %1728 = add nuw i64 %1713, 1
  %1729 = icmp eq i64 %1728, %1708
  br i1 %1729, label %.loopexit265, label %1709

.loopexit265:                                     ; preds = %1724, %1700
  %1730 = phi i64 [ 0, %1700 ], [ %1727, %1724 ]
  %1731 = phi i64 [ 0, %1700 ], [ %1726, %1724 ]
  %1732 = phi i64 [ 0, %1700 ], [ %1725, %1724 ]
  br i1 %1368, label %1736, label %.loopexit263

.loopexit263:                                     ; preds = %1748, %1736, %.loopexit265
  %1733 = phi i64 [ 0, %.loopexit265 ], [ 0, %1736 ], [ %1750, %1748 ]
  %1734 = load atomic i32, ptr %1409 acquire, align 8, !noalias !34359
  %1735 = icmp eq i32 %1734, 0
  br i1 %1735, label %1752, label %1755

1736:                                             ; preds = %.loopexit265
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %1600) ]
  %1737 = ptrtoint ptr %1600 to i64
  %1738 = call i64 @llvm.usub.sat.i64(i64 %1607, i64 %1601)
  %1739 = sub nuw i64 %1500, %1737
  %1740 = udiv exact i64 %1739, 88
  %1741 = call i64 @llvm.umin.i64(i64 %1738, i64 %1740)
  %1742 = icmp eq i64 %1741, 0
  br i1 %1742, label %.loopexit263, label %.preheader262

.preheader262:                                    ; preds = %1736, %1748
  %1743 = phi i64 [ %1750, %1748 ], [ 0, %1736 ]
  %1744 = phi i64 [ %1749, %1748 ], [ 0, %1736 ]
  %1745 = getelementptr inbounds nuw [88 x i8], ptr %1600, i64 %1744
  %1746 = getelementptr inbounds nuw i8, ptr %1745, i64 8
; invoke purrdf_sparql_eval::scratch::value_bytes
  %1747 = invoke noundef i64 @purrdf_sparql_eval::scratch::value_bytes(ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(80) %1746)
          to label %1748 unwind label %1582, !noalias !34230

1748:                                             ; preds = %.preheader262
  %1749 = add nuw nsw i64 %1744, 1
  %1750 = call noundef range(i64 32, 0) i64 @llvm.uadd.sat.i64(i64 %1743, i64 %1747)
  %1751 = icmp eq i64 %1749, %1741
  br i1 %1751, label %.loopexit263, label %.preheader262

1752:                                             ; preds = %.loopexit263
  %1753 = load i8, ptr %1410, align 8, !noalias !34230
  %1754 = icmp eq i8 %1753, -1
  br i1 %1754, label %1755, label %1762

1755:                                             ; preds = %1752, %.loopexit263
  br i1 %1365, label %1756, label %1757

1756:                                             ; preds = %1757, %1755
  br i1 %1368, label %1765, label %1763

1757:                                             ; preds = %1755
  %1758 = load atomic i64, ptr %1402 monotonic, align 8, !noalias !34362
  %1759 = load i64, ptr %1363, align 8, !noalias !34362
  %1760 = call i64 @llvm.uadd.sat.i64(i64 %1758, i64 %1730)
  %1761 = icmp ugt i64 %1760, %1759
  br i1 %1761, label %1762, label %1756

1762:                                             ; preds = %1765, %1757, %1752
  br i1 %1705, label %.loopexit254, label %.preheader252

1763:                                             ; preds = %1765, %1756
  %1764 = or i1 %1365, %1705
  br i1 %1764, label %.loopexit261, label %.preheader260

1765:                                             ; preds = %1756
  %1766 = load i64, ptr %1411, align 8, !alias.scope !34229, !noalias !34230, !noundef !1708
  %1767 = load atomic i64, ptr %1412 monotonic, align 16, !alias.scope !34229, !noalias !34230
  %1768 = call noundef i64 @llvm.usub.sat.i64(i64 %1766, i64 %1767)
  %1769 = call i64 @llvm.uadd.sat.i64(i64 %1768, i64 %1733)
  %1770 = call i64 @llvm.uadd.sat.i64(i64 %1769, i64 %1731)
  %1771 = call i64 @llvm.uadd.sat.i64(i64 %1770, i64 %1732)
  %1772 = load atomic i64, ptr %1413 monotonic, align 8, !noalias !34365
  %1773 = load i64, ptr %1366, align 8, !noalias !34365
  %1774 = call i64 @llvm.uadd.sat.i64(i64 %1772, i64 %1771)
  %1775 = icmp ugt i64 %1774, %1773
  br i1 %1775, label %1762, label %1763

.preheader252:                                    ; preds = %1762, %1958
  %1776 = phi ptr [ %1959, %1958 ], [ %1597, %1762 ]
  %1777 = phi ptr [ %1781, %1958 ], [ %1702, %1762 ]
  %1778 = phi ptr [ %1962, %1958 ], [ %1600, %1762 ]
  %1779 = phi i64 [ %1961, %1958 ], [ %1601, %1762 ]
  %1780 = phi ptr [ %1960, %1958 ], [ %1597, %1762 ]
  %1781 = getelementptr inbounds nuw i8, ptr %1777, i64 24
  %1782 = load i8, ptr %1777, align 8, !range !10848, !noalias !34230, !noundef !1708
  switch i8 %1782, label %default.unreachable1172 [
    i8 0, label %1788
    i8 1, label %1795
    i8 2, label %1800
    i8 3, label %1823
  ]

.loopexit254:                                     ; preds = %1958, %1762
  %1783 = phi ptr [ %1597, %1762 ], [ %1959, %1958 ]
  %1784 = phi i64 [ %1601, %1762 ], [ %1961, %1958 ]
  %1785 = phi ptr [ %1600, %1762 ], [ %1962, %1958 ]
  %1786 = icmp ult i64 %1784, %1607
  %1787 = select i1 %1368, i1 %1786, i1 false
  br i1 %1787, label %1978, label %1971

1788:                                             ; preds = %.preheader252
  %1789 = getelementptr inbounds nuw i8, ptr %1777, i64 1
  %1790 = load i8, ptr %1789, align 1, !range !1905, !noalias !34230, !noundef !1708
  %1791 = getelementptr inbounds nuw i8, ptr %1777, i64 8
  %1792 = load i64, ptr %1791, align 8, !noalias !34230, !noundef !1708
  %1793 = getelementptr inbounds nuw i8, ptr %1777, i64 16
  %1794 = load i64, ptr %1793, align 8, !noalias !34230, !noundef !1708
  br i1 %1365, label %1958, label %1824

1795:                                             ; preds = %.preheader252
  %1796 = getelementptr inbounds nuw i8, ptr %1777, i64 8
  %1797 = load i64, ptr %1796, align 8, !noalias !34230, !noundef !1708
  %1798 = getelementptr inbounds nuw i8, ptr %1777, i64 16
  %1799 = load i64, ptr %1798, align 8, !noalias !34230, !noundef !1708
  br i1 %1368, label %1883, label %1958

1800:                                             ; preds = %.preheader252
  %1801 = getelementptr inbounds nuw i8, ptr %1777, i64 8
  %1802 = load i64, ptr %1801, align 8, !noalias !34230, !noundef !1708
  %1803 = icmp ult i64 %1779, %1802
  br i1 %1803, label %1804, label %1935

1804:                                             ; preds = %1800
  %1805 = icmp eq ptr %1778, %1489
  br i1 %1805, label %.loopexit247, label %1806

1806:                                             ; preds = %1804
  %1807 = add i64 %1802, -1
  br label %1811

1808:                                             ; preds = %1821
  %1809 = add i64 %1813, 1
  %1810 = icmp eq ptr %1814, %1489
  br i1 %1810, label %.loopexit247, label %1811

1811:                                             ; preds = %1808, %1806
  %1812 = phi ptr [ %1814, %1808 ], [ %1778, %1806 ]
  %1813 = phi i64 [ %1809, %1808 ], [ %1779, %1806 ]
  %1814 = getelementptr inbounds nuw i8, ptr %1812, i64 88
  %1815 = getelementptr inbounds nuw i8, ptr %1812, i64 8
  %1816 = load i64, ptr %1815, align 8, !noalias !34368
  %1817 = icmp eq i64 %1816, -1
  br i1 %1817, label %.loopexit247, label %1818

1818:                                             ; preds = %1811
  %1819 = getelementptr inbounds nuw i8, ptr %1812, i64 16
  %1820 = load i64, ptr %1812, align 8, !noalias !34368
  call void @llvm.lifetime.start.p0(ptr nonnull %54), !noalias !34371
  store i64 %1816, ptr %54, align 8, !noalias !34371
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %1417, ptr noundef nonnull align 8 dereferenceable(72) %1819, i64 72, i1 false), !noalias !34230
; invoke <purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint
  invoke void @<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint(ptr noalias nofree noundef nonnull align 8 dereferenceable(184) %1418, i64 noundef %1820, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(80) %54)
          to label %1821 unwind label %1570, !noalias !34230

1821:                                             ; preds = %1818
  call void @llvm.lifetime.end.p0(ptr nonnull %54), !noalias !34371
  %1822 = icmp eq i64 %1813, %1807
  br i1 %1822, label %.loopexit247, label %1808

1823:                                             ; preds = %.preheader252
  br i1 %1368, label %1944, label %1958

1824:                                             ; preds = %1788
  call void @llvm.lifetime.start.p0(ptr nonnull %53), !noalias !34374
  %1825 = load atomic i64, ptr %1402 monotonic, align 8, !noalias !34381
  br label %1826

1826:                                             ; preds = %1826, %1824
  %1827 = phi i64 [ %1825, %1824 ], [ %1831, %1826 ]
  %1828 = call i64 @llvm.uadd.sat.i64(i64 %1827, i64 %1792)
  %1829 = cmpxchg weak ptr %1402, i64 %1827, i64 %1828 monotonic monotonic, align 8, !noalias !34381
  %1830 = extractvalue { i64, i1 } %1829, 1
  %1831 = extractvalue { i64, i1 } %1829, 0
  br i1 %1830, label %1832, label %1826

1832:                                             ; preds = %1826
  %1833 = call i64 @llvm.uadd.sat.i64(i64 %1831, i64 %1792)
  %1834 = load i64, ptr %1363, align 8, !noalias !34381
  %1835 = icmp ugt i64 %1833, %1834
  br i1 %1835, label %1836, label %1845

1836:                                             ; preds = %1832
  call void @llvm.lifetime.start.p0(ptr nonnull %52), !noalias !34384
  store i8 0, ptr %1423, align 1, !noalias !34384
  store i64 %1834, ptr %1424, align 8, !noalias !34384
  store i64 %1833, ptr %1425, align 8, !noalias !34384
  store i8 0, ptr %52, align 8, !noalias !34384
; invoke <purrdf_sparql_eval::governor::GovernorState>::trip
  invoke fastcc void @<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.13412714042204560522)(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(24) %53, ptr noundef nonnull align 8 %1363, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %52)
          to label %1837 unwind label %1566, !noalias !34230

1837:                                             ; preds = %1836
  call void @llvm.lifetime.end.p0(ptr nonnull %52), !noalias !34384
  %1838 = load i8, ptr %53, align 8, !noalias !34374
  %1839 = icmp eq i8 %1838, -1
  br i1 %1839, label %1845, label %1840

1840:                                             ; preds = %1837
  call void @llvm.lifetime.end.p0(ptr nonnull %53), !noalias !34374
  %1841 = icmp ne i64 %1794, 0
  %1842 = add i64 %1794, -1
  %1843 = icmp ult i64 %1779, %1842
  %1844 = select i1 %1841, i1 %1843, i1 false
  br i1 %1844, label %1847, label %.loopexit253

1845:                                             ; preds = %1837, %1832
  call void @llvm.lifetime.end.p0(ptr nonnull %53), !noalias !34374
  %1846 = icmp eq i8 %1790, -1
  br i1 %1846, label %1958, label %1866

1847:                                             ; preds = %1840
  %1848 = icmp eq ptr %1778, %1489
  br i1 %1848, label %.loopexit245, label %1849

1849:                                             ; preds = %1847
  %1850 = add i64 %1794, -2
  br label %1851

1851:                                             ; preds = %1861, %1849
  %1852 = phi ptr [ %1854, %1861 ], [ %1778, %1849 ]
  %1853 = phi i64 [ %1863, %1861 ], [ %1779, %1849 ]
  %1854 = getelementptr inbounds nuw i8, ptr %1852, i64 88
  %1855 = getelementptr inbounds nuw i8, ptr %1852, i64 8
  %1856 = load i64, ptr %1855, align 8, !noalias !34385
  %1857 = icmp eq i64 %1856, -1
  br i1 %1857, label %.loopexit245, label %1858

1858:                                             ; preds = %1851
  %1859 = getelementptr inbounds nuw i8, ptr %1852, i64 16
  %1860 = load i64, ptr %1852, align 8, !noalias !34385
  call void @llvm.lifetime.start.p0(ptr nonnull %51), !noalias !34388
  store i64 %1856, ptr %51, align 8, !noalias !34388
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %1426, ptr noundef nonnull align 8 dereferenceable(72) %1859, i64 72, i1 false), !noalias !34230
; invoke <purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint
  invoke void @<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint(ptr noalias nofree noundef nonnull align 8 dereferenceable(184) %1418, i64 noundef %1860, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(80) %51)
          to label %1861 unwind label %1564, !noalias !34230

1861:                                             ; preds = %1858
  call void @llvm.lifetime.end.p0(ptr nonnull %51), !noalias !34388
  %1862 = icmp eq i64 %1853, %1850
  %1863 = add nuw i64 %1853, 1
  %1864 = icmp eq ptr %1854, %1489
  %1865 = select i1 %1862, i1 true, i1 %1864
  br i1 %1865, label %.loopexit245, label %1851

1866:                                             ; preds = %1845
  %1867 = load ptr, ptr %1406, align 8, !alias.scope !34229, !noalias !34230, !noundef !1708
  %1868 = icmp eq ptr %1867, null
  br i1 %1868, label %1958, label %1869

1869:                                             ; preds = %1866
  %1870 = load i32, ptr %1407, align 4, !alias.scope !34229, !noalias !34230, !noundef !1708
  %1871 = getelementptr i8, ptr %1867, i64 56
  %1872 = load i64, ptr %1871, align 8, !noalias !34230, !noundef !1708
  %1873 = zext i32 %1870 to i64
  %1874 = icmp ugt i64 %1872, %1873
  br i1 %1874, label %1875, label %1958

1875:                                             ; preds = %1869
  %1876 = getelementptr i8, ptr %1867, i64 48
  %1877 = load ptr, ptr %1876, align 8, !noalias !34230, !nonnull !1708, !noundef !1708
  %1878 = zext nneg i8 %1790 to i64
  %1879 = getelementptr inbounds nuw [136 x i8], ptr %1877, i64 %1873
  %1880 = getelementptr inbounds nuw [8 x i8], ptr %1879, i64 %1878
  %1881 = atomicrmw add ptr %1880, i64 %1792 monotonic, align 8, !noalias !34230
  br label %1958

1882:                                             ; preds = %1887
  br i1 %1889, label %.loopexit253, label %1958

1883:                                             ; preds = %1795
  call void @llvm.lifetime.start.p0(ptr nonnull %65), !noalias !34208
  %1884 = load i64, ptr %1366, align 8, !noalias !34230
  %1885 = icmp eq i64 %1884, -1
  br i1 %1885, label %1886, label %1892

1886:                                             ; preds = %1903, %1883
  call void @llvm.lifetime.end.p0(ptr nonnull %65), !noalias !34208
  br label %1958

1887:                                             ; preds = %1909, %1907
  %1888 = load i8, ptr %65, align 8, !noalias !34208
  %1889 = icmp ne i8 %1888, -1
  call void @llvm.lifetime.end.p0(ptr nonnull %65), !noalias !34208
  %1890 = icmp ne i64 %1799, 0
  %1891 = and i1 %1890, %1889
  br i1 %1891, label %1910, label %1882

1892:                                             ; preds = %1883
  %1893 = load atomic i32, ptr %1409 acquire, align 8, !noalias !34391
  %1894 = icmp eq i32 %1893, 0
  br i1 %1894, label %1907, label %1895

1895:                                             ; preds = %1892
  %1896 = load atomic i64, ptr %1413 monotonic, align 8, !noalias !34391
  br label %1897

1897:                                             ; preds = %1897, %1895
  %1898 = phi i64 [ %1896, %1895 ], [ %1902, %1897 ]
  %1899 = call i64 @llvm.uadd.sat.i64(i64 %1898, i64 %1797)
  %1900 = cmpxchg weak ptr %1413, i64 %1898, i64 %1899 monotonic monotonic, align 8, !noalias !34391
  %1901 = extractvalue { i64, i1 } %1900, 1
  %1902 = extractvalue { i64, i1 } %1900, 0
  br i1 %1901, label %1903, label %1897

1903:                                             ; preds = %1897
  %1904 = call i64 @llvm.uadd.sat.i64(i64 %1902, i64 %1797)
  %1905 = load i64, ptr %1366, align 8, !noalias !34391
  %1906 = icmp ugt i64 %1904, %1905
  br i1 %1906, label %1908, label %1886

1907:                                             ; preds = %1892
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %65, ptr noundef nonnull align 8 dereferenceable(24) %1410, i64 24, i1 false), !noalias !34230
  br label %1887

1908:                                             ; preds = %1903
  call void @llvm.lifetime.start.p0(ptr nonnull %43), !noalias !34394
  store i8 3, ptr %1419, align 1, !noalias !34394
  store i64 %1905, ptr %1420, align 8, !noalias !34394
  store i64 %1904, ptr %1421, align 8, !noalias !34394
  store i8 0, ptr %43, align 8, !noalias !34394
; invoke <purrdf_sparql_eval::governor::GovernorState>::trip
  invoke fastcc void @<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.13412714042204560522)(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(24) %65, ptr noundef nonnull align 8 %1363, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %43)
          to label %1909 unwind label %1580, !noalias !34230

1909:                                             ; preds = %1908
  call void @llvm.lifetime.end.p0(ptr nonnull %43), !noalias !34394
  br label %1887

1910:                                             ; preds = %1887
  %1911 = add i64 %1799, -1
  %1912 = icmp ult i64 %1779, %1911
  br i1 %1912, label %1913, label %.loopexit253

1913:                                             ; preds = %1910
  %1914 = icmp eq ptr %1778, %1489
  br i1 %1914, label %.loopexit245, label %1915

1915:                                             ; preds = %1913
  %1916 = add i64 %1799, -2
  br label %1917

1917:                                             ; preds = %1927, %1915
  %1918 = phi ptr [ %1920, %1927 ], [ %1778, %1915 ]
  %1919 = phi i64 [ %1929, %1927 ], [ %1779, %1915 ]
  %1920 = getelementptr inbounds nuw i8, ptr %1918, i64 88
  %1921 = getelementptr inbounds nuw i8, ptr %1918, i64 8
  %1922 = load i64, ptr %1921, align 8, !noalias !34395
  %1923 = icmp eq i64 %1922, -1
  br i1 %1923, label %.loopexit245, label %1924

1924:                                             ; preds = %1917
  %1925 = getelementptr inbounds nuw i8, ptr %1918, i64 16
  %1926 = load i64, ptr %1918, align 8, !noalias !34395
  call void @llvm.lifetime.start.p0(ptr nonnull %50), !noalias !34398
  store i64 %1922, ptr %50, align 8, !noalias !34398
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %1422, ptr noundef nonnull align 8 dereferenceable(72) %1925, i64 72, i1 false), !noalias !34230
; invoke <purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint
  invoke void @<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint(ptr noalias nofree noundef nonnull align 8 dereferenceable(184) %1418, i64 noundef %1926, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(80) %50)
          to label %1927 unwind label %1568, !noalias !34230

1927:                                             ; preds = %1924
  call void @llvm.lifetime.end.p0(ptr nonnull %50), !noalias !34398
  %1928 = icmp eq i64 %1919, %1916
  %1929 = add nuw i64 %1919, 1
  %1930 = icmp eq ptr %1920, %1489
  %1931 = select i1 %1928, i1 true, i1 %1930
  br i1 %1931, label %.loopexit245, label %1917

.loopexit247:                                     ; preds = %1821, %1811, %1808, %1804
  %1932 = phi ptr [ %1780, %1804 ], [ %1814, %1808 ], [ %1814, %1811 ], [ %1814, %1821 ]
  %1933 = phi i64 [ %1779, %1804 ], [ %1802, %1821 ], [ %1813, %1811 ], [ %1809, %1808 ]
  %1934 = phi ptr [ %1778, %1804 ], [ %1814, %1808 ], [ %1814, %1811 ], [ %1814, %1821 ]
  store ptr %1932, ptr %1396, align 8, !noalias !34208
  br label %1935

1935:                                             ; preds = %.loopexit247, %1800
  %1936 = phi ptr [ %1776, %1800 ], [ %1932, %.loopexit247 ]
  %1937 = phi ptr [ %1780, %1800 ], [ %1932, %.loopexit247 ]
  %1938 = phi i64 [ %1779, %1800 ], [ %1933, %.loopexit247 ]
  %1939 = phi ptr [ %1778, %1800 ], [ %1934, %.loopexit247 ]
  br i1 %1368, label %1940, label %1958

1940:                                             ; preds = %1935
  call void @llvm.lifetime.start.p0(ptr nonnull %64), !noalias !34208
; invoke <purrdf_sparql_eval::eval::EvalCtx>::charge_scratch_growth
  invoke fastcc void @<purrdf_sparql_eval::eval::EvalCtx>::charge_scratch_growth(ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %64, ptr noundef nonnull align 16 dereferenceable(1248) %4)
          to label %1941 unwind label %1580, !noalias !34230

1941:                                             ; preds = %1940
  %1942 = load i8, ptr %64, align 8, !range !1906, !noalias !34208, !noundef !1708
  %1943 = icmp eq i8 %1942, -1
  call void @llvm.lifetime.end.p0(ptr nonnull %64), !noalias !34208
  br i1 %1943, label %1958, label %.loopexit253

1944:                                             ; preds = %1823
  %1945 = getelementptr inbounds nuw i8, ptr %1777, i64 8
  %1946 = load i64, ptr %1945, align 8, !noalias !34230, !noundef !1708
  call void @llvm.lifetime.start.p0(ptr nonnull %63), !noalias !34208
  %1947 = load atomic i32, ptr %1409 acquire, align 8, !noalias !34401
  %1948 = icmp eq i32 %1947, 0
  br i1 %1948, label %1949, label %1950

1949:                                             ; preds = %1944
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %63, ptr noundef nonnull align 8 dereferenceable(24) %1410, i64 24, i1 false), !noalias !34230
  br label %1964

1950:                                             ; preds = %1944
  %1951 = load atomic i64, ptr %1413 monotonic, align 8, !noalias !34401
  %1952 = call i64 @llvm.uadd.sat.i64(i64 %1951, i64 %1946)
  %1953 = load i64, ptr %1366, align 8, !noalias !34401
  %1954 = icmp ugt i64 %1952, %1953
  br i1 %1954, label %1956, label %1955

1955:                                             ; preds = %1950
  call void @llvm.lifetime.end.p0(ptr nonnull %63), !noalias !34208
  br label %1958

1956:                                             ; preds = %1950
  call void @llvm.lifetime.start.p0(ptr nonnull %49), !noalias !34404
  store i8 3, ptr %1414, align 1, !noalias !34404
  store i64 %1953, ptr %1415, align 8, !noalias !34404
  store i64 %1952, ptr %1416, align 8, !noalias !34404
  store i8 0, ptr %49, align 8, !noalias !34404
; invoke <purrdf_sparql_eval::governor::GovernorState>::trip
  invoke fastcc void @<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.13412714042204560522)(ptr noalias nofree noundef align 8 captures(none) dereferenceable(24) %63, ptr noundef nonnull align 8 %1363, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %49)
          to label %1957 unwind label %1580, !noalias !34230

1957:                                             ; preds = %1956
  call void @llvm.lifetime.end.p0(ptr nonnull %49), !noalias !34404
  br label %1964

1958:                                             ; preds = %1964, %1955, %1941, %1935, %1886, %1882, %1875, %1869, %1866, %1845, %1823, %1795, %1788
  %1959 = phi ptr [ %1776, %1882 ], [ %1936, %1935 ], [ %1776, %1823 ], [ %1776, %1788 ], [ %1776, %1795 ], [ %1776, %1955 ], [ %1936, %1941 ], [ %1776, %1964 ], [ %1776, %1845 ], [ %1776, %1866 ], [ %1776, %1875 ], [ %1776, %1869 ], [ %1776, %1886 ]
  %1960 = phi ptr [ %1780, %1882 ], [ %1937, %1935 ], [ %1780, %1823 ], [ %1780, %1788 ], [ %1780, %1795 ], [ %1780, %1955 ], [ %1937, %1941 ], [ %1780, %1964 ], [ %1780, %1845 ], [ %1780, %1866 ], [ %1780, %1875 ], [ %1780, %1869 ], [ %1780, %1886 ]
  %1961 = phi i64 [ %1779, %1882 ], [ %1938, %1935 ], [ %1779, %1823 ], [ %1779, %1788 ], [ %1779, %1795 ], [ %1779, %1955 ], [ %1938, %1941 ], [ %1779, %1964 ], [ %1779, %1845 ], [ %1779, %1866 ], [ %1779, %1875 ], [ %1779, %1869 ], [ %1779, %1886 ]
  %1962 = phi ptr [ %1778, %1882 ], [ %1939, %1935 ], [ %1778, %1823 ], [ %1778, %1788 ], [ %1778, %1795 ], [ %1778, %1955 ], [ %1939, %1941 ], [ %1778, %1964 ], [ %1778, %1845 ], [ %1778, %1866 ], [ %1778, %1875 ], [ %1778, %1869 ], [ %1778, %1886 ]
  %1963 = icmp eq ptr %1781, %1704
  br i1 %1963, label %.loopexit254, label %.preheader252

1964:                                             ; preds = %1957, %1949
  %1965 = load i8, ptr %63, align 8, !noalias !34208
  %1966 = icmp eq i8 %1965, -1
  call void @llvm.lifetime.end.p0(ptr nonnull %63), !noalias !34208
  br i1 %1966, label %1958, label %.loopexit253

.loopexit245:                                     ; preds = %1927, %1917, %1861, %1851, %1913, %1847
  %1967 = phi ptr [ %1780, %1913 ], [ %1780, %1847 ], [ %1854, %1861 ], [ %1854, %1851 ], [ %1920, %1917 ], [ %1920, %1927 ]
  store ptr %1967, ptr %1396, align 8, !noalias !34208
  br label %.loopexit253

.loopexit250:                                     ; preds = %1995, %1985, %1982, %1978
  %1968 = phi ptr [ %1783, %1978 ], [ %1988, %1982 ], [ %1988, %1985 ], [ %1988, %1995 ]
  %1969 = phi i64 [ %1784, %1978 ], [ %1607, %1995 ], [ %1987, %1985 ], [ %1983, %1982 ]
  %1970 = phi ptr [ %1785, %1978 ], [ %1988, %1982 ], [ %1988, %1985 ], [ %1988, %1995 ]
  store ptr %1968, ptr %1396, align 8, !noalias !34208
  br label %1971

1971:                                             ; preds = %2089, %2064, %.loopexit250, %.loopexit254
  %1972 = phi ptr [ %2059, %2064 ], [ %2090, %2089 ], [ %1783, %.loopexit254 ], [ %1968, %.loopexit250 ]
  %1973 = phi i64 [ %2060, %2064 ], [ %2091, %2089 ], [ %1784, %.loopexit254 ], [ %1969, %.loopexit250 ]
  %1974 = phi ptr [ %2061, %2064 ], [ %2092, %2089 ], [ %1785, %.loopexit254 ], [ %1970, %.loopexit250 ]
  %1975 = icmp eq i64 %1609, 0
  br i1 %1975, label %.loopexit248, label %1976

1976:                                             ; preds = %1971
  %1977 = load ptr, ptr %1260, align 8, !alias.scope !34405, !noalias !34408, !nonnull !1708, !noundef !1708
  br label %2148

1978:                                             ; preds = %.loopexit254
  %1979 = icmp eq ptr %1785, %1489
  br i1 %1979, label %.loopexit250, label %1980

1980:                                             ; preds = %1978
  %1981 = add i64 %1607, -1
  br label %1985

1982:                                             ; preds = %1995
  %1983 = add i64 %1987, 1
  %1984 = icmp eq ptr %1988, %1489
  br i1 %1984, label %.loopexit250, label %1985

1985:                                             ; preds = %1982, %1980
  %1986 = phi ptr [ %1988, %1982 ], [ %1785, %1980 ]
  %1987 = phi i64 [ %1983, %1982 ], [ %1784, %1980 ]
  %1988 = getelementptr inbounds nuw i8, ptr %1986, i64 88
  %1989 = getelementptr inbounds nuw i8, ptr %1986, i64 8
  %1990 = load i64, ptr %1989, align 8, !noalias !34410
  %1991 = icmp eq i64 %1990, -1
  br i1 %1991, label %.loopexit250, label %1992

1992:                                             ; preds = %1985
  %1993 = getelementptr inbounds nuw i8, ptr %1986, i64 16
  %1994 = load i64, ptr %1986, align 8, !noalias !34410
  call void @llvm.lifetime.start.p0(ptr nonnull %48), !noalias !34413
  store i64 %1990, ptr %48, align 8, !noalias !34413
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %1427, ptr noundef nonnull align 8 dereferenceable(72) %1993, i64 72, i1 false), !noalias !34230
; invoke <purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint
  invoke void @<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint(ptr noalias nofree noundef nonnull align 8 dereferenceable(184) %1418, i64 noundef %1994, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(80) %48)
          to label %1995 unwind label %1578, !noalias !34230

1995:                                             ; preds = %1992
  call void @llvm.lifetime.end.p0(ptr nonnull %48), !noalias !34413
  %1996 = icmp eq i64 %1987, %1981
  br i1 %1996, label %.loopexit250, label %1982

.loopexit261:                                     ; preds = %2012, %1763
  %1997 = icmp eq i64 %1598, %1697
  br i1 %1997, label %.loopexit259, label %.lr.ph

1998:                                             ; preds = %.lr.ph
  %1999 = icmp eq ptr %1702, %2001
  br i1 %1999, label %.loopexit259, label %.lr.ph

.lr.ph:                                           ; preds = %.loopexit261, %1998
  %2000 = phi ptr [ %2001, %1998 ], [ %1704, %.loopexit261 ]
  %2001 = getelementptr inbounds i8, ptr %2000, i64 -24
  %2002 = load i8, ptr %2001, align 8, !range !10848, !noalias !34416, !noundef !1708
  %2003 = icmp eq i8 %2002, 2
  br i1 %2003, label %2032, label %1998

.preheader260:                                    ; preds = %1763, %2012
  %2004 = phi ptr [ %2005, %2012 ], [ %1702, %1763 ]
  %2005 = getelementptr inbounds nuw i8, ptr %2004, i64 24
  %2006 = load i8, ptr %2004, align 8, !range !10848, !noalias !34230, !noundef !1708
  %2007 = icmp eq i8 %2006, 0
  br i1 %2007, label %2008, label %2012

2008:                                             ; preds = %.preheader260
  %2009 = getelementptr inbounds nuw i8, ptr %2004, i64 1
  %2010 = load i8, ptr %2009, align 1, !range !1905, !noalias !34230, !noundef !1708
  %2011 = icmp eq i8 %2010, -1
  br i1 %2011, label %2012, label %2014

2012:                                             ; preds = %2023, %2017, %2014, %2008, %.preheader260
  %2013 = icmp eq ptr %2005, %1704
  br i1 %2013, label %.loopexit261, label %.preheader260

2014:                                             ; preds = %2008
  %2015 = load ptr, ptr %1406, align 8, !alias.scope !34229, !noalias !34230, !noundef !1708
  %2016 = icmp eq ptr %2015, null
  br i1 %2016, label %2012, label %2017

2017:                                             ; preds = %2014
  %2018 = load i32, ptr %1407, align 4, !alias.scope !34229, !noalias !34230, !noundef !1708
  %2019 = getelementptr i8, ptr %2015, i64 56
  %2020 = load i64, ptr %2019, align 8, !noalias !34230, !noundef !1708
  %2021 = zext i32 %2018 to i64
  %2022 = icmp ugt i64 %2020, %2021
  br i1 %2022, label %2023, label %2012

2023:                                             ; preds = %2017
  %2024 = getelementptr i8, ptr %2015, i64 48
  %2025 = load ptr, ptr %2024, align 8, !noalias !34230, !nonnull !1708, !noundef !1708
  %2026 = getelementptr inbounds nuw i8, ptr %2004, i64 8
  %2027 = load i64, ptr %2026, align 8, !noalias !34230, !noundef !1708
  %2028 = zext nneg i8 %2010 to i64
  %2029 = getelementptr inbounds nuw [136 x i8], ptr %2025, i64 %2021
  %2030 = getelementptr inbounds nuw [8 x i8], ptr %2029, i64 %2028
  %2031 = atomicrmw add ptr %2030, i64 %2027 monotonic, align 8, !noalias !34230
  br label %2012

2032:                                             ; preds = %.lr.ph
  %2033 = getelementptr i8, ptr %2000, i64 -16
  %2034 = load i64, ptr %2033, align 8, !noalias !34416
  %2035 = icmp ult i64 %1601, %2034
  br i1 %2035, label %2036, label %.loopexit259

2036:                                             ; preds = %2032
  %2037 = icmp eq ptr %1600, %1489
  br i1 %2037, label %.loopexit257, label %2038

2038:                                             ; preds = %2036
  %2039 = add i64 %2034, -1
  br label %2043

2040:                                             ; preds = %2053
  %2041 = add i64 %2045, 1
  %2042 = icmp eq ptr %2046, %1489
  br i1 %2042, label %.loopexit257, label %2043

2043:                                             ; preds = %2040, %2038
  %2044 = phi ptr [ %2046, %2040 ], [ %1600, %2038 ]
  %2045 = phi i64 [ %2041, %2040 ], [ %1601, %2038 ]
  %2046 = getelementptr inbounds nuw i8, ptr %2044, i64 88
  %2047 = getelementptr inbounds nuw i8, ptr %2044, i64 8
  %2048 = load i64, ptr %2047, align 8, !noalias !34419
  %2049 = icmp eq i64 %2048, -1
  br i1 %2049, label %.loopexit257, label %2050

2050:                                             ; preds = %2043
  %2051 = getelementptr inbounds nuw i8, ptr %2044, i64 16
  %2052 = load i64, ptr %2044, align 8, !noalias !34419
  call void @llvm.lifetime.start.p0(ptr nonnull %47), !noalias !34422
  store i64 %2048, ptr %47, align 8, !noalias !34422
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %1428, ptr noundef nonnull align 8 dereferenceable(72) %2051, i64 72, i1 false), !noalias !34230
; invoke <purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint
  invoke void @<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint(ptr noalias nofree noundef nonnull align 8 dereferenceable(184) %1418, i64 noundef %2052, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(80) %47)
          to label %2053 unwind label %1576, !noalias !34230

2053:                                             ; preds = %2050
  call void @llvm.lifetime.end.p0(ptr nonnull %47), !noalias !34422
  %2054 = icmp eq i64 %2045, %2039
  br i1 %2054, label %.loopexit257, label %2040

.loopexit257:                                     ; preds = %2053, %2043, %2040, %2036
  %2055 = phi ptr [ %1597, %2036 ], [ %2046, %2040 ], [ %2046, %2043 ], [ %2046, %2053 ]
  %2056 = phi i64 [ %1601, %2036 ], [ %2034, %2053 ], [ %2045, %2043 ], [ %2041, %2040 ]
  %2057 = phi ptr [ %1600, %2036 ], [ %2046, %2040 ], [ %2046, %2043 ], [ %2046, %2053 ]
  store ptr %2055, ptr %1396, align 8, !noalias !34208
  br label %.loopexit259

.loopexit259:                                     ; preds = %1998, %.loopexit261, %.loopexit257, %2032
  %2058 = phi i1 [ false, %.loopexit257 ], [ false, %2032 ], [ true, %.loopexit261 ], [ true, %1998 ]
  %2059 = phi ptr [ %2055, %.loopexit257 ], [ %1597, %2032 ], [ %1597, %.loopexit261 ], [ %1597, %1998 ]
  %2060 = phi i64 [ %2056, %.loopexit257 ], [ %1601, %2032 ], [ %1601, %.loopexit261 ], [ %1601, %1998 ]
  %2061 = phi ptr [ %2057, %.loopexit257 ], [ %1600, %2032 ], [ %1600, %.loopexit261 ], [ %1600, %1998 ]
  %2062 = icmp eq i64 %1730, 0
  %2063 = or i1 %1365, %2062
  br i1 %2063, label %2064, label %2065

2064:                                             ; preds = %2081, %.loopexit259
  br i1 %1368, label %2083, label %1971

2065:                                             ; preds = %.loopexit259
  call void @llvm.lifetime.start.p0(ptr nonnull %46), !noalias !34425
  %2066 = load atomic i64, ptr %1402 monotonic, align 8, !noalias !34432
  br label %2067

2067:                                             ; preds = %2067, %2065
  %2068 = phi i64 [ %2066, %2065 ], [ %2072, %2067 ]
  %2069 = call i64 @llvm.uadd.sat.i64(i64 %2068, i64 %1730)
  %2070 = cmpxchg weak ptr %1402, i64 %2068, i64 %2069 monotonic monotonic, align 8, !noalias !34432
  %2071 = extractvalue { i64, i1 } %2070, 1
  %2072 = extractvalue { i64, i1 } %2070, 0
  br i1 %2071, label %2073, label %2067

2073:                                             ; preds = %2067
  %2074 = call i64 @llvm.uadd.sat.i64(i64 %2072, i64 %1730)
  %2075 = load i64, ptr %1363, align 8, !noalias !34432
  %2076 = icmp ugt i64 %2074, %2075
  br i1 %2076, label %2077, label %2081

2077:                                             ; preds = %2073
  call void @llvm.lifetime.start.p0(ptr nonnull %45), !noalias !34435
  store i8 0, ptr %1429, align 1, !noalias !34435
  store i64 %2075, ptr %1430, align 8, !noalias !34435
  store i64 %2074, ptr %1431, align 8, !noalias !34435
  store i8 0, ptr %45, align 8, !noalias !34435
; invoke <purrdf_sparql_eval::governor::GovernorState>::trip
  invoke fastcc void @<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.13412714042204560522)(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(24) %46, ptr noundef nonnull align 8 %1363, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %45)
          to label %2078 unwind label %1574, !noalias !34230

2078:                                             ; preds = %2077
  call void @llvm.lifetime.end.p0(ptr nonnull %45), !noalias !34435
  %2079 = load i8, ptr %46, align 8, !noalias !34425
  %2080 = icmp eq i8 %2079, -1
  br i1 %2080, label %2081, label %2082

2081:                                             ; preds = %2078, %2073
  call void @llvm.lifetime.end.p0(ptr nonnull %46), !noalias !34425
  br label %2064

2082:                                             ; preds = %2078
  call void @llvm.lifetime.end.p0(ptr nonnull %46), !noalias !34425
  br i1 %1368, label %2126, label %.loopexit253

2083:                                             ; preds = %2064
  call void @llvm.lifetime.start.p0(ptr nonnull %68), !noalias !34208
  %2084 = load i64, ptr %1366, align 8, !noalias !34230
  %2085 = icmp eq i64 %2084, -1
  br i1 %2085, label %2111, label %2093

.loopexit255:                                     ; preds = %2146, %2136, %2133, %2129
  %2086 = phi ptr [ %2059, %2129 ], [ %2139, %2133 ], [ %2139, %2136 ], [ %2139, %2146 ]
  %2087 = phi i64 [ %2060, %2129 ], [ %1607, %2146 ], [ %2138, %2136 ], [ %2134, %2133 ]
  %2088 = phi ptr [ %2061, %2129 ], [ %2139, %2133 ], [ %2139, %2136 ], [ %2139, %2146 ]
  store ptr %2086, ptr %1396, align 8, !noalias !34208
  br label %2089

2089:                                             ; preds = %2126, %.loopexit255
  %2090 = phi ptr [ %2059, %2126 ], [ %2086, %.loopexit255 ]
  %2091 = phi i64 [ %2060, %2126 ], [ %2087, %.loopexit255 ]
  %2092 = phi ptr [ %2061, %2126 ], [ %2088, %.loopexit255 ]
  br i1 %2127, label %.loopexit253, label %1971

2093:                                             ; preds = %2083
  %2094 = load atomic i32, ptr %1409 acquire, align 8, !noalias !34436
  %2095 = icmp eq i32 %2094, 0
  br i1 %2095, label %2108, label %2096

2096:                                             ; preds = %2093
  %2097 = load atomic i64, ptr %1413 monotonic, align 8, !noalias !34436
  br label %2098

2098:                                             ; preds = %2098, %2096
  %2099 = phi i64 [ %2097, %2096 ], [ %2103, %2098 ]
  %2100 = call i64 @llvm.uadd.sat.i64(i64 %2099, i64 %1731)
  %2101 = cmpxchg weak ptr %1413, i64 %2099, i64 %2100 monotonic monotonic, align 8, !noalias !34436
  %2102 = extractvalue { i64, i1 } %2101, 1
  %2103 = extractvalue { i64, i1 } %2101, 0
  br i1 %2102, label %2104, label %2098

2104:                                             ; preds = %2098
  %2105 = call i64 @llvm.uadd.sat.i64(i64 %2103, i64 %1731)
  %2106 = load i64, ptr %1366, align 8, !noalias !34436
  %2107 = icmp ugt i64 %2105, %2106
  br i1 %2107, label %2109, label %2111

2108:                                             ; preds = %2093
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %68, ptr noundef nonnull align 8 dereferenceable(24) %1410, i64 24, i1 false), !noalias !34230
  br label %2112

2109:                                             ; preds = %2104
  call void @llvm.lifetime.start.p0(ptr nonnull %40), !noalias !34439
  store i8 3, ptr %1437, align 1, !noalias !34439
  store i64 %2106, ptr %1438, align 8, !noalias !34439
  store i64 %2105, ptr %1439, align 8, !noalias !34439
  store i8 0, ptr %40, align 8, !noalias !34439
; invoke <purrdf_sparql_eval::governor::GovernorState>::trip
  invoke fastcc void @<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.13412714042204560522)(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(24) %68, ptr noundef nonnull align 8 %1363, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %40)
          to label %2110 unwind label %1586, !noalias !34171

2110:                                             ; preds = %2109
  call void @llvm.lifetime.end.p0(ptr nonnull %40), !noalias !34439
  br label %2112

2111:                                             ; preds = %2112, %2104, %2083
  call void @llvm.lifetime.end.p0(ptr nonnull %68), !noalias !34208
  br i1 %2058, label %2116, label %2118

2112:                                             ; preds = %2110, %2108
  %2113 = load i8, ptr %68, align 8, !noalias !34208
  %2114 = icmp eq i8 %2113, -1
  br i1 %2114, label %2111, label %2115

2115:                                             ; preds = %2112
  call void @llvm.lifetime.end.p0(ptr nonnull %68), !noalias !34208
  br label %2126

2116:                                             ; preds = %2119, %2111
  %2117 = icmp eq i64 %1732, 0
  br i1 %2117, label %2126, label %2122

2118:                                             ; preds = %2111
  call void @llvm.lifetime.start.p0(ptr nonnull %67), !noalias !34208
; invoke <purrdf_sparql_eval::eval::EvalCtx>::charge_scratch_growth
  invoke fastcc void @<purrdf_sparql_eval::eval::EvalCtx>::charge_scratch_growth(ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %67, ptr noundef nonnull align 16 dereferenceable(1248) %4)
          to label %2119 unwind label %1586, !noalias !34230

2119:                                             ; preds = %2118
  %2120 = load i8, ptr %67, align 8, !range !1906, !noalias !34208, !noundef !1708
  %2121 = icmp eq i8 %2120, -1
  call void @llvm.lifetime.end.p0(ptr nonnull %67), !noalias !34208
  br i1 %2121, label %2116, label %2126

2122:                                             ; preds = %2116
  call void @llvm.lifetime.start.p0(ptr nonnull %66), !noalias !34208
; invoke <purrdf_sparql_eval::governor::GovernorState>::admit_transient
  invoke void @<purrdf_sparql_eval::governor::GovernorState>::admit_transient(ptr noalias nofree noundef nonnull sret([24 x i8]) align 8 captures(none) dereferenceable(24) %66, ptr noundef nonnull align 8 %1363, i8 noundef 3, i64 noundef %1732)
          to label %2123 unwind label %1586, !noalias !34230

2123:                                             ; preds = %2122
  %2124 = load i8, ptr %66, align 8, !range !1906, !noalias !34208, !noundef !1708
  %2125 = icmp ne i8 %2124, -1
  call void @llvm.lifetime.end.p0(ptr nonnull %66), !noalias !34208
  br label %2126

2126:                                             ; preds = %2123, %2119, %2116, %2115, %2082
  %2127 = phi i1 [ false, %2116 ], [ %2125, %2123 ], [ true, %2082 ], [ true, %2115 ], [ true, %2119 ]
  %2128 = icmp ult i64 %2060, %1607
  br i1 %2128, label %2129, label %2089

2129:                                             ; preds = %2126
  %2130 = icmp eq ptr %2061, %1489
  br i1 %2130, label %.loopexit255, label %2131

2131:                                             ; preds = %2129
  %2132 = add i64 %1607, -1
  br label %2136

2133:                                             ; preds = %2146
  %2134 = add i64 %2138, 1
  %2135 = icmp eq ptr %2139, %1489
  br i1 %2135, label %.loopexit255, label %2136

2136:                                             ; preds = %2133, %2131
  %2137 = phi ptr [ %2139, %2133 ], [ %2061, %2131 ]
  %2138 = phi i64 [ %2134, %2133 ], [ %2060, %2131 ]
  %2139 = getelementptr inbounds nuw i8, ptr %2137, i64 88
  %2140 = getelementptr inbounds nuw i8, ptr %2137, i64 8
  %2141 = load i64, ptr %2140, align 8, !noalias !34440
  %2142 = icmp eq i64 %2141, -1
  br i1 %2142, label %.loopexit255, label %2143

2143:                                             ; preds = %2136
  %2144 = getelementptr inbounds nuw i8, ptr %2137, i64 16
  %2145 = load i64, ptr %2137, align 8, !noalias !34440
  call void @llvm.lifetime.start.p0(ptr nonnull %44), !noalias !34443
  store i64 %2141, ptr %44, align 8, !noalias !34443
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %1432, ptr noundef nonnull align 8 dereferenceable(72) %2144, i64 72, i1 false), !noalias !34230
; invoke <purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint
  invoke void @<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint(ptr noalias nofree noundef nonnull align 8 dereferenceable(184) %1418, i64 noundef %2145, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(80) %44)
          to label %2146 unwind label %1572, !noalias !34230

2146:                                             ; preds = %2143
  call void @llvm.lifetime.end.p0(ptr nonnull %44), !noalias !34443
  %2147 = icmp eq i64 %2138, %2132
  br i1 %2147, label %.loopexit255, label %2133

2148:                                             ; preds = %2173, %1976
  %2149 = phi i64 [ %1609, %1976 ], [ %2151, %2173 ]
  %2150 = phi ptr [ %1596, %1976 ], [ %2156, %2173 ]
  %2151 = add i64 %2149, -1
  call void @llvm.experimental.noalias.scope.decl(metadata !34405)
  %2152 = icmp eq ptr %2150, %1977
  br i1 %2152, label %.loopexit248, label %2155

.loopexit248:                                     ; preds = %2173, %2155, %2148, %1971
  %2153 = phi ptr [ %1596, %1971 ], [ %2156, %2155 ], [ %2156, %2173 ], [ %2150, %2148 ]
  store ptr %2153, ptr %1259, align 8, !noalias !34208
  %2154 = icmp eq ptr %1602, %1495
  br i1 %2154, label %.loopexit267, label %1595

2155:                                             ; preds = %2148
  %2156 = getelementptr inbounds nuw i8, ptr %2150, i64 40
  %2157 = load i64, ptr %2150, align 8, !noalias !34446
  %2158 = getelementptr inbounds nuw i8, ptr %2150, i64 8
  %2159 = load ptr, ptr %2158, align 8, !noalias !34446
  %2160 = icmp eq i64 %2157, 0
  br i1 %2160, label %.loopexit248, label %2161

2161:                                             ; preds = %2155
  %2162 = getelementptr inbounds nuw i8, ptr %2150, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %62, ptr noundef nonnull align 8 dereferenceable(24) %2162, i64 24, i1 false), !noalias !34230
  call void @llvm.experimental.noalias.scope.decl(metadata !34447)
  %2163 = load i64, ptr %1433, align 8, !alias.scope !34447, !noalias !34450, !noundef !1708
  %2164 = load i64, ptr %79, align 8, !range !1817, !alias.scope !34447, !noalias !34450, !noundef !1708
  %2165 = icmp eq i64 %2163, %2164
  br i1 %2165, label %2166, label %2173

2166:                                             ; preds = %2161
; invoke <alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one
  invoke void @<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %79)
          to label %2173 unwind label %2167, !noalias !34452

2167:                                             ; preds = %2166
  %2168 = landingpad { ptr, i32 }
          cleanup
  store ptr %2156, ptr %1259, align 8, !noalias !34208
  %2169 = icmp ugt i64 %2157, 5
  br i1 %2169, label %2170, label %1590

2170:                                             ; preds = %2167
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %2159) ]
  %2171 = shl i64 %2157, 3
  %2172 = add i64 %2171, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %2159, i64 noundef %2172, i64 noundef range(i64 1, -9223372036854775807) 4) #88, !noalias !34453
  br label %1590

2173:                                             ; preds = %2166, %2161
  %2174 = load ptr, ptr %1434, align 8, !alias.scope !34447, !noalias !34450, !nonnull !1708, !noundef !1708
  %2175 = getelementptr inbounds nuw [40 x i8], ptr %2174, i64 %2163
  store i64 %2157, ptr %2175, align 8, !noalias !34456
  %2176 = getelementptr inbounds nuw i8, ptr %2175, i64 8
  store ptr %2159, ptr %2176, align 8, !noalias !34456
  %2177 = getelementptr inbounds nuw i8, ptr %2175, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %2177, ptr noundef nonnull align 8 dereferenceable(24) %62, i64 24, i1 false), !noalias !34456
  %2178 = add i64 %2163, 1
  store i64 %2178, ptr %1433, align 8, !alias.scope !34447, !noalias !34450
  %2179 = icmp eq i64 %2151, 0
  br i1 %2179, label %.loopexit248, label %2148

.loopexit253:                                     ; preds = %2089, %2082, %1964, %1941, %1882, %.loopexit245, %1910, %1840, %1681
  %2180 = phi i8 [ 0, %1681 ], [ 1, %1840 ], [ 1, %1910 ], [ 1, %.loopexit245 ], [ 1, %1964 ], [ 1, %1882 ], [ 1, %1941 ], [ 1, %2082 ], [ 1, %2089 ]
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %85, ptr noundef nonnull align 8 dereferenceable(24) %79, i64 24, i1 false), !noalias !34214
  %2181 = icmp eq i64 %1491, 0
  br i1 %2181, label %2185, label %2182

2182:                                             ; preds = %.loopexit253
  %2183 = shl nuw i64 %1491, 5
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %1490, i64 noundef %2183, i64 noundef range(i64 1, -9223372036854775807) 8) #88, !noalias !34457
  br label %2185

2184:                                             ; preds = %1699
  unreachable

2185:                                             ; preds = %2182, %.loopexit253
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>(ptr noalias nofree noundef align 8 dereferenceable(32) %69)
          to label %2186 unwind label %1626, !noalias !34230

2186:                                             ; preds = %2185
  call void @llvm.lifetime.end.p0(ptr nonnull %69), !noalias !34208
  %2187 = icmp eq i64 %1483, 0
  br i1 %2187, label %2190, label %2188

2188:                                             ; preds = %2186
  %2189 = mul nuw i64 %1483, 24
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %1484) ]
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %1484, i64 noundef %2189, i64 noundef range(i64 1, -9223372036854775807) 8) #88, !noalias !34230
  br label %2190

2190:                                             ; preds = %2188, %2186
  call void @llvm.experimental.noalias.scope.decl(metadata !34460)
  %2191 = load ptr, ptr %1435, align 8, !alias.scope !34460, !noalias !34208, !noundef !1708
  %2192 = icmp eq ptr %2191, null
  br i1 %2192, label %2197, label %2193

2193:                                             ; preds = %2190
  %2194 = atomicrmw sub ptr %2191, i64 1 release, align 8, !noalias !34463
  %2195 = icmp eq i64 %2194, 1
  br i1 %2195, label %2196, label %2197

2196:                                             ; preds = %2193
  fence acquire
; call <alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %1435) #87, !noalias !34230
  br label %2197

2197:                                             ; preds = %2196, %2193, %2190
  call void @llvm.experimental.noalias.scope.decl(metadata !34468)
  %2198 = load ptr, ptr %1436, align 8, !alias.scope !34468, !noalias !34208, !noundef !1708
  %2199 = icmp eq ptr %2198, null
  br i1 %2199, label %2204, label %2200

2200:                                             ; preds = %2197
  %2201 = atomicrmw sub ptr %2198, i64 1 release, align 8, !noalias !34471
  %2202 = icmp eq i64 %2201, 1
  br i1 %2202, label %2203, label %2204

2203:                                             ; preds = %2200
  fence acquire
; call <alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %1436) #87, !noalias !34230
  br label %2204

2204:                                             ; preds = %2203, %2200, %2197
  call void @llvm.lifetime.end.p0(ptr nonnull %70), !noalias !34208
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>(ptr noalias nofree noundef align 8 dereferenceable(32) %71)
          to label %2205 unwind label %1456, !noalias !34230

2205:                                             ; preds = %2204
  call void @llvm.lifetime.end.p0(ptr nonnull %71), !noalias !34208
  %2206 = atomicrmw sub ptr %1362, i64 1 release, align 8, !noalias !34476
  %2207 = icmp eq i64 %2206, 1
  br i1 %2207, label %2208, label %2209

2208:                                             ; preds = %2205
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %72) #87
          to label %2209 unwind label %1337, !noalias !34230

2209:                                             ; preds = %2208, %2205
  call void @llvm.lifetime.end.p0(ptr nonnull %72), !noalias !34208
  call void @llvm.experimental.noalias.scope.decl(metadata !34481)
  call void @llvm.experimental.noalias.scope.decl(metadata !34484)
  %2210 = load ptr, ptr %1259, align 8, !alias.scope !34487, !noalias !34208, !nonnull !1708, !noundef !1708
  %2211 = load ptr, ptr %1260, align 8, !alias.scope !34487, !noalias !34208, !nonnull !1708, !noundef !1708
  %2212 = ptrtoint ptr %2211 to i64
  %2213 = ptrtoint ptr %2210 to i64
  %2214 = sub nuw i64 %2212, %2213
  %2215 = udiv exact i64 %2214, 40
  call void @llvm.experimental.noalias.scope.decl(metadata !34488)
  %2216 = icmp eq ptr %2211, %2210
  br i1 %2216, label %.loopexit244, label %.preheader243

.preheader243:                                    ; preds = %2209
  %2217 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %2218 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  br label %2219

2219:                                             ; preds = %.preheader243, %2257
  %2220 = phi i64 [ %2222, %2257 ], [ 0, %.preheader243 ]
  %2221 = getelementptr inbounds nuw [40 x i8], ptr %2210, i64 %2220
  %2222 = add nuw nsw i64 %2220, 1
  %2223 = load i64, ptr %2221, align 8, !range !1940, !alias.scope !34491, !noalias !34494, !noundef !1708
  %2224 = icmp ugt i64 %2223, 5
  br i1 %2224, label %2225, label %2257

2225:                                             ; preds = %2219
  %2226 = getelementptr i8, ptr %2221, i64 8
  %2227 = load ptr, ptr %2226, align 8, !alias.scope !34488, !noalias !34494, !nonnull !1708, !noundef !1708
  %2228 = shl i64 %2223, 3
  %2229 = add i64 %2228, -8
  %2230 = load i64, ptr %2217, align 8, !noalias !34495, !noundef !1708
  %2231 = call i64 @llvm.umin.i64(i64 %2229, i64 9223372036854775807)
  %2232 = call i64 @llvm.ssub.sat.i64(i64 %2230, i64 %2231)
  store i64 %2232, ptr %2217, align 8, !noalias !34495
  %2233 = load i64, ptr %2218, align 8, !noalias !34495, !noundef !1708
  %2234 = icmp slt i64 %2232, %2233
  br i1 %2234, label %2235, label %.preheader1955

2235:                                             ; preds = %2225
  store i64 %2232, ptr %2218, align 8, !noalias !34495
  br label %.preheader1955

.preheader1955:                                   ; preds = %2235, %2225
  br label %2236

2236:                                             ; preds = %.preheader1955, %2239
  %2237 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952) acquire, align 8, !noalias !34495
  %2238 = icmp slt i64 %2237, 0
  br i1 %2238, label %2239, label %__rustc::__rust_dealloc (.exit220)

2239:                                             ; preds = %2236
  %2240 = add nsw i64 %2237, 1
  %2241 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 %2237, i64 %2240 acq_rel acquire, align 8, !noalias !34495
  %2242 = extractvalue { i64, i1 } %2241, 1
  br i1 %2242, label %2243, label %2236

2243:                                             ; preds = %2239
  %2244 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %2231 monotonic, align 8, !noalias !34495
  %2245 = call i64 @llvm.ssub.sat.i64(i64 %2244, i64 %2231)
  %2246 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !34495
  br label %2247

2247:                                             ; preds = %2250, %2243
  %2248 = phi i64 [ %2246, %2243 ], [ %2253, %2250 ]
  %2249 = icmp slt i64 %2245, %2248
  br i1 %2249, label %2250, label %2254

2250:                                             ; preds = %2247
  %2251 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %2248, i64 %2245 monotonic monotonic, align 8, !noalias !34495
  %2252 = extractvalue { i64, i1 } %2251, 1
  %2253 = extractvalue { i64, i1 } %2251, 0
  br i1 %2252, label %2254, label %2247

2254:                                             ; preds = %2250, %2247
  %2255 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 1 release, align 8, !noalias !34495
  br label %__rustc::__rust_dealloc (.exit220)

__rustc::__rust_dealloc (.exit220): ; preds = %2236, %2254
  %2256 = icmp ne i64 %2229, 0
  call void @llvm.assume(i1 %2256), !noalias !34495
  call void @free(ptr noundef nonnull %2227) #88, !noalias !34495
  br label %2257

2257:                                             ; preds = %__rustc::__rust_dealloc (.exit220), %2219
  %2258 = icmp eq i64 %2222, %2215
  br i1 %2258, label %.loopexit244, label %2219

.loopexit244:                                     ; preds = %2257, %2209
  %2259 = load i64, ptr %1258, align 8, !alias.scope !34487, !noalias !34208, !noundef !1708
  %2260 = icmp eq i64 %2259, 0
  br i1 %2260, label %2338, label %2333

2261:                                             ; preds = %1638, %1635, %1632
  call void @llvm.experimental.noalias.scope.decl(metadata !34498)
  %2262 = load ptr, ptr %1436, align 8, !alias.scope !34498, !noalias !34208, !noundef !1708
  %2263 = icmp eq ptr %2262, null
  br i1 %2263, label %1474, label %2264

2264:                                             ; preds = %2261
  %2265 = atomicrmw sub ptr %2262, i64 1 release, align 8, !noalias !34501
  %2266 = icmp eq i64 %2265, 1
  br i1 %2266, label %2267, label %1474

2267:                                             ; preds = %2264
  fence acquire
; call <alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %1436) #87, !noalias !34230
  br label %1474

2268:                                             ; preds = %1455, %1449
; call core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(32) %75) #89, !noalias !34230
  br i1 %1450, label %2269, label %2270

2269:                                             ; preds = %2268
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %79) #89, !noalias !34230
  br i1 %1451, label %2331, label %2988

2270:                                             ; preds = %2268
  br i1 %1451, label %2331, label %2988

2271:                                             ; preds = %1339, %1251, %.loopexit272
  %2272 = phi i56 [ undef, %1339 ], [ %1200, %.loopexit272 ], [ %1200, %1251 ]
  %2273 = phi i64 [ undef, %1339 ], [ %1202, %.loopexit272 ], [ %1202, %1251 ]
  %2274 = phi i64 [ -1, %1339 ], [ %1193, %.loopexit272 ], [ %1193, %1251 ]
  %2275 = phi i8 [ 2, %1339 ], [ %1198, %.loopexit272 ], [ %1198, %1251 ]
  %2276 = icmp eq i64 %1135, 0
  br i1 %2276, label %._crit_edge1766, label %.lr.ph1765

2277:                                             ; preds = %.lr.ph1765
  %2278 = icmp eq i64 %2281, %1135
  br i1 %2278, label %._crit_edge1766, label %.lr.ph1765

.lr.ph1765:                                       ; preds = %2271, %2277
  %2279 = phi i64 [ %2281, %2277 ], [ 0, %2271 ]
  %2280 = getelementptr inbounds nuw [160 x i8], ptr %1136, i64 %2279
  %2281 = add nuw nsw i64 %2279, 1
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>(ptr noalias nofree noundef readonly align 8 dereferenceable(160) %2280)
          to label %2277 unwind label %2285, !noalias !34506

2282:                                             ; preds = %.lr.ph1768
  %2283 = add i64 %2288, 1
  %2284 = icmp eq i64 %2283, %1135
  br i1 %2284, label %._crit_edge1769, label %.lr.ph1768

2285:                                             ; preds = %.lr.ph1765
  %2286 = landingpad { ptr, i32 }
          cleanup
  %2287 = icmp eq i64 %2281, %1135
  br i1 %2287, label %._crit_edge1769, label %.lr.ph1768

.lr.ph1768:                                       ; preds = %2285, %2282
  %2288 = phi i64 [ %2283, %2282 ], [ %2281, %2285 ]
  %2289 = getelementptr inbounds nuw [160 x i8], ptr %1136, i64 %2288
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>(ptr noalias nofree noundef readonly align 8 dereferenceable(160) %2289) #89
          to label %2282 unwind label %2290, !noalias !34506

2290:                                             ; preds = %.lr.ph1768
  %2291 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #86, !noalias !34509
  unreachable

._crit_edge1769:                                  ; preds = %2282, %2285
  %2292 = icmp eq i64 %1156, 0
  br i1 %2292, label %2988, label %2293

2293:                                             ; preds = %._crit_edge1769
  %2294 = mul nuw i64 %1156, 160
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %1136, i64 noundef %2294, i64 noundef range(i64 1, -9223372036854775807) 8) #88, !noalias !34506
  br label %2988

._crit_edge1766:                                  ; preds = %2277, %2271
  %2295 = icmp eq i64 %1156, 0
  br i1 %2295, label %2340, label %2296

2296:                                             ; preds = %._crit_edge1766
  %2297 = mul nuw i64 %1156, 160
  %2298 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %2299 = load i64, ptr %2298, align 8, !noalias !34506, !noundef !1708
  %2300 = call i64 @llvm.umin.i64(i64 %2297, i64 9223372036854775807)
  %2301 = call i64 @llvm.ssub.sat.i64(i64 %2299, i64 %2300)
  store i64 %2301, ptr %2298, align 8, !noalias !34506
  %2302 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %2303 = load i64, ptr %2302, align 8, !noalias !34506, !noundef !1708
  %2304 = icmp slt i64 %2301, %2303
  br i1 %2304, label %2305, label %.preheader1944

2305:                                             ; preds = %2296
  store i64 %2301, ptr %2302, align 8, !noalias !34506
  br label %.preheader1944

.preheader1944:                                   ; preds = %2305, %2296
  br label %2306

2306:                                             ; preds = %.preheader1944, %2309
  %2307 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952) acquire, align 8, !noalias !34506
  %2308 = icmp slt i64 %2307, 0
  br i1 %2308, label %2309, label %__rustc::__rust_dealloc (.exit221)

2309:                                             ; preds = %2306
  %2310 = add nsw i64 %2307, 1
  %2311 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 %2307, i64 %2310 acq_rel acquire, align 8, !noalias !34506
  %2312 = extractvalue { i64, i1 } %2311, 1
  br i1 %2312, label %2313, label %2306

2313:                                             ; preds = %2309
  %2314 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %2300 monotonic, align 8, !noalias !34506
  %2315 = call i64 @llvm.ssub.sat.i64(i64 %2314, i64 %2300)
  %2316 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !34506
  br label %2317

2317:                                             ; preds = %2320, %2313
  %2318 = phi i64 [ %2316, %2313 ], [ %2323, %2320 ]
  %2319 = icmp slt i64 %2315, %2318
  br i1 %2319, label %2320, label %2324

2320:                                             ; preds = %2317
  %2321 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %2318, i64 %2315 monotonic monotonic, align 8, !noalias !34506
  %2322 = extractvalue { i64, i1 } %2321, 1
  %2323 = extractvalue { i64, i1 } %2321, 0
  br i1 %2322, label %2324, label %2317

2324:                                             ; preds = %2320, %2317
  %2325 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 1 release, align 8, !noalias !34506
  br label %__rustc::__rust_dealloc (.exit221)

__rustc::__rust_dealloc (.exit221): ; preds = %2306, %2324
  call void @free(ptr noundef nonnull %1136) #88, !noalias !34506
  br label %2340

2326:                                             ; preds = %.loopexit274
  %2327 = landingpad { ptr, i32 }
          cleanup
  br label %2328

2328:                                             ; preds = %2326, %1275
  %2329 = phi ptr [ %119, %2326 ], [ %79, %1275 ]
  %2330 = phi { ptr, i32 } [ %2327, %2326 ], [ %1347, %1275 ]
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %2329) #89
  br label %2331

2331:                                             ; preds = %2328, %2270, %2269
  %2332 = phi { ptr, i32 } [ %1452, %2270 ], [ %1452, %2269 ], [ %2330, %2328 ]
; invoke core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(32) %84) #89
          to label %2988 unwind label %1359, !noalias !34512

2333:                                             ; preds = %.loopexit244, %.loopexit242
  %2334 = phi i64 [ %1562, %.loopexit242 ], [ %2259, %.loopexit244 ]
  %2335 = phi i8 [ 2, %.loopexit242 ], [ %2180, %.loopexit244 ]
  %2336 = load ptr, ptr %75, align 8, !noalias !34300, !nonnull !1708, !noundef !1708
  %2337 = mul nuw i64 %2334, 40
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %2336, i64 noundef %2337, i64 noundef range(i64 1, -9223372036854775807) 8) #88, !noalias !34230
  br label %2338

2338:                                             ; preds = %2333, %.loopexit244, %.loopexit242
  %2339 = phi i8 [ %2180, %.loopexit244 ], [ 2, %.loopexit242 ], [ %2335, %2333 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %75), !noalias !34208
  call void @llvm.lifetime.end.p0(ptr nonnull %79), !noalias !34208
  call void @llvm.lifetime.end.p0(ptr nonnull %62)
  call void @llvm.lifetime.end.p0(ptr nonnull %73)
  call void @llvm.lifetime.end.p0(ptr nonnull %84), !noalias !34160
  br label %2342

2340:                                             ; preds = %__rustc::__rust_dealloc (.exit221), %._crit_edge1766
  call void @llvm.lifetime.end.p0(ptr nonnull %62)
  call void @llvm.lifetime.end.p0(ptr nonnull %73)
  call void @llvm.lifetime.end.p0(ptr nonnull %84), !noalias !34160
  %2341 = icmp eq i64 %2274, -1
  br i1 %2341, label %2342, label %2384

2342:                                             ; preds = %2340, %2338
  %2343 = phi i8 [ %2339, %2338 ], [ %2275, %2340 ]
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %87, ptr noundef nonnull align 8 dereferenceable(24) %85, i64 24, i1 false), !noalias !34160
  call void @llvm.lifetime.end.p0(ptr nonnull %85)
  call void @llvm.lifetime.end.p0(ptr nonnull %86)
  call void @llvm.lifetime.start.p0(ptr nonnull %83), !noalias !34160
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %83, ptr noundef nonnull align 8 dereferenceable(24) %87, i64 24, i1 false), !noalias !34160
  call void @llvm.lifetime.end.p0(ptr nonnull %87)
  call void @llvm.lifetime.start.p0(ptr nonnull %42)
  %2344 = load ptr, ptr %555, align 8, !alias.scope !34173, !noalias !34171, !noundef !1708
  %2345 = icmp eq ptr %2344, null
  br i1 %2345, label %2356, label %2346

2346:                                             ; preds = %2342
  %2347 = getelementptr inbounds nuw i8, ptr %2344, i64 296
  %2348 = load atomic i32, ptr %2347 acquire, align 4, !noalias !34513
  %2349 = icmp eq i32 %2348, 0
  br i1 %2349, label %2350, label %2354

2350:                                             ; preds = %2346
  %2351 = getelementptr inbounds nuw i8, ptr %2344, i64 272
  %2352 = load i8, ptr %2351, align 8, !noalias !34171
  %2353 = getelementptr inbounds nuw i8, ptr %2344, i64 273
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(23) %42, ptr noundef nonnull align 1 dereferenceable(23) %2353, i64 23, i1 false), !noalias !34171
  br label %2354

2354:                                             ; preds = %2350, %2346
  %2355 = phi i8 [ %2352, %2350 ], [ -1, %2346 ]
  switch i8 %2343, label %2360 [
    i8 2, label %2377
    i8 0, label %2359
  ]

2356:                                             ; preds = %2342
  %2357 = icmp eq i8 %2343, 2
  %2358 = and i1 %2357, %1181
  br label %2377

2359:                                             ; preds = %2374, %2362, %2354
  br label %2377

2360:                                             ; preds = %2354
  %2361 = icmp eq i8 %2355, -1
  br i1 %2361, label %2377, label %2362

2362:                                             ; preds = %2360
  %2363 = load i8, ptr %160, align 8, !range !3634, !alias.scope !34173, !noalias !34171, !noundef !1708
  %2364 = icmp eq i8 %2363, 2
  br i1 %2364, label %2365, label %2359

2365:                                             ; preds = %2362
  %2366 = getelementptr inbounds nuw i8, ptr %4, i64 696
  call void @llvm.experimental.noalias.scope.decl(metadata !34516)
  %2367 = load ptr, ptr %2366, align 8, !alias.scope !34519, !noalias !34520, !nonnull !1708, !noundef !1708
  call void @llvm.lifetime.start.p0(ptr nonnull %41), !noalias !34522
  store i8 %2355, ptr %41, align 8, !noalias !34526
  %2368 = getelementptr inbounds nuw i8, ptr %41, i64 1
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(23) %2368, ptr noundef nonnull align 1 dereferenceable(23) %42, i64 23, i1 false), !noalias !34160
  %2369 = getelementptr inbounds nuw i8, ptr %2367, i64 40
  %2370 = load atomic i32, ptr %2369 acquire, align 4, !noalias !34527
  %2371 = icmp eq i32 %2370, 0
  br i1 %2371, label %2374, label %2372, !prof !1974

2372:                                             ; preds = %2365
  %2373 = getelementptr inbounds nuw i8, ptr %2367, i64 16
; invoke <std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::initialize::<<std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::get_or_init<<std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::try_insert::{closure#0}>::{closure#0}, !>
  invoke fastcc void @<std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::initialize::<<std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::get_or_init<<std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::try_insert::{closure#0}>::{closure#0}, !> (.llvm.13412714042204560522)(ptr noundef nonnull align 8 %2373, ptr noundef nonnull align 8 %41)
          to label %2374 unwind label %2375, !noalias !34171

2374:                                             ; preds = %2372, %2365
  call void @llvm.lifetime.end.p0(ptr nonnull %41), !noalias !34522
  br label %2359

2375:                                             ; preds = %2372
  %2376 = landingpad { ptr, i32 }
          cleanup
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %83) #89, !noalias !34171
  br label %2988

2377:                                             ; preds = %2360, %2359, %2356, %2354
  %2378 = phi i8 [ -1, %2356 ], [ -1, %2360 ], [ %2355, %2359 ], [ %2355, %2354 ]
  %2379 = phi i1 [ %2358, %2356 ], [ false, %2360 ], [ false, %2359 ], [ %1181, %2354 ]
  %2380 = icmp eq i8 %2378, -1
  %2381 = select i1 %2379, i1 %2380, i1 false
  call void @llvm.lifetime.end.p0(ptr nonnull %42)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %108, ptr noundef nonnull align 8 dereferenceable(24) %83, i64 24, i1 false), !noalias !34528
  call void @llvm.lifetime.end.p0(ptr nonnull %83), !noalias !34160
  call void @llvm.lifetime.end.p0(ptr nonnull %89), !noalias !34160
  br label %2394

2382:                                             ; preds = %1107, %1018
  %2383 = phi { ptr, i32 } [ %1108, %1107 ], [ %1019, %1018 ]
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(24) %107) #89, !noalias !34529
  br label %2988

2384:                                             ; preds = %2340
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %87, ptr noundef nonnull align 8 dereferenceable(24) %85, i64 24, i1 false), !noalias !34160
  %2385 = getelementptr inbounds nuw i8, ptr %0, i64 64
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(48) %2385, ptr noundef nonnull align 1 dereferenceable(48) %86, i64 48, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %85)
  call void @llvm.lifetime.end.p0(ptr nonnull %86)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %108, ptr noundef nonnull align 8 dereferenceable(24) %87, i64 24, i1 false), !noalias !34528
  call void @llvm.lifetime.end.p0(ptr nonnull %87)
  call void @llvm.lifetime.end.p0(ptr nonnull %89), !noalias !34160
  call void @llvm.lifetime.end.p0(ptr nonnull %107)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %109, ptr noundef nonnull align 8 dereferenceable(24) %108, i64 24, i1 false)
  %2386 = zext i56 %2272 to i64
  %2387 = shl nuw i64 %2386, 8
  %2388 = zext i8 %2275 to i64
  %2389 = or disjoint i64 %2387, %2388
  call void @llvm.lifetime.end.p0(ptr nonnull %108)
  %2390 = getelementptr inbounds nuw i8, ptr %0, i64 24
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %2390, ptr noundef nonnull align 8 dereferenceable(24) %109, i64 24, i1 false)
  %2391 = getelementptr inbounds nuw i8, ptr %0, i64 16
  store i64 %2274, ptr %2391, align 16
  %2392 = getelementptr inbounds nuw i8, ptr %0, i64 48
  store i64 %2389, ptr %2392, align 16
  %2393 = getelementptr inbounds nuw i8, ptr %0, i64 56
  store i64 %2273, ptr %2393, align 8
  store i64 1, ptr %0, align 16
  call void @llvm.lifetime.end.p0(ptr nonnull %109)
  call void @llvm.lifetime.end.p0(ptr nonnull %113)
; invoke core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>>(ptr noalias nofree noundef align 8 dereferenceable(32) %114)
          to label %2728 unwind label %988

2394:                                             ; preds = %2377, %998
  %2395 = phi i1 [ false, %998 ], [ %2381, %2377 ]
  %2396 = phi i64 [ undef, %998 ], [ %1180, %2377 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %107)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %109, ptr noundef nonnull align 8 dereferenceable(24) %108, i64 24, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %108)
  call void @llvm.lifetime.start.p0(ptr nonnull %110)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %110, ptr noundef nonnull align 8 dereferenceable(24) %109, i64 24, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %109)
  %2397 = load i64, ptr %114, align 8
  %2398 = getelementptr inbounds nuw i8, ptr %114, i64 8
  %2399 = load i64, ptr %2398, align 8
  %2400 = getelementptr inbounds nuw i8, ptr %114, i64 16
  %2401 = load i64, ptr %2400, align 8
  %2402 = getelementptr inbounds nuw i8, ptr %114, i64 24
  %2403 = load i64, ptr %2402, align 8
  %2404 = icmp ugt i64 %2397, 2
  %2405 = select i1 %2404, i64 %2401, i64 %2397
  %2406 = add i64 %2405, -1
  %2407 = select i1 %2404, i64 %2397, i64 1
  %2408 = select i1 %2404, i64 1, i64 %2401
  call void @llvm.lifetime.start.p0(ptr nonnull %39), !noalias !34530
  store i64 %2407, ptr %39, align 8, !noalias !34534
  %2409 = getelementptr inbounds nuw i8, ptr %39, i64 8
  store i64 %2399, ptr %2409, align 8, !noalias !34534
  %2410 = getelementptr inbounds nuw i8, ptr %39, i64 16
  store i64 %2408, ptr %2410, align 8, !noalias !34534
  %2411 = getelementptr inbounds nuw i8, ptr %39, i64 24
  store i64 %2403, ptr %2411, align 8, !noalias !34534
  %2412 = getelementptr inbounds nuw i8, ptr %39, i64 32
  store i64 0, ptr %2412, align 8, !noalias !34530
  %2413 = getelementptr inbounds nuw i8, ptr %39, i64 40
  store i64 %2406, ptr %2413, align 8, !noalias !34530
  %2414 = icmp eq i64 %2406, 0
  br i1 %2414, label %.loopexit237, label %2415

2415:                                             ; preds = %2394
  %2416 = inttoptr i64 %2399 to ptr
  %2417 = select i1 %2404, ptr %2416, ptr %2409
  %2418 = getelementptr inbounds nuw i8, ptr %4, i64 640
  br label %2421

2419:                                             ; preds = %2421
  %2420 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<purrdf_core::small::IntoIter<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_core::small::IntoIter<[purrdf_sparql_eval::witness::RelationWitness; 1]>>(ptr noalias nofree noundef align 8 dereferenceable(48) %39) #89
          to label %2429 unwind label %2427, !noalias !34535

2421:                                             ; preds = %2425, %2415
  %2422 = phi i64 [ 0, %2415 ], [ %2423, %2425 ]
  %2423 = add nuw i64 %2422, 1
  store i64 %2423, ptr %2412, align 8, !alias.scope !34536, !noalias !34539
  %2424 = getelementptr inbounds nuw [24 x i8], ptr %2417, i64 %2422
  call void @llvm.lifetime.start.p0(ptr nonnull %38), !noalias !34530
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %38, ptr noundef nonnull align 8 dereferenceable(24) %2424, i64 24, i1 false), !noalias !34535
; invoke <purrdf_sparql_eval::witness::RelationWitness>::merge
  invoke void @<purrdf_sparql_eval::witness::RelationWitness>::merge(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %2418, ptr noalias nofree noundef nonnull readonly align 8 captures(none) dereferenceable(24) %38)
          to label %2425 unwind label %2419, !noalias !34535

.loopexit237:                                     ; preds = %2425, %2394
; invoke core::ptr::drop_glue::<purrdf_core::small::IntoIter<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_core::small::IntoIter<[purrdf_sparql_eval::witness::RelationWitness; 1]>>(ptr noalias nofree noundef align 8 dereferenceable(48) %39)
          to label %2433 unwind label %2431

2425:                                             ; preds = %2421
  call void @llvm.lifetime.end.p0(ptr nonnull %38), !noalias !34530
  %2426 = icmp eq i64 %2423, %2406
  br i1 %2426, label %.loopexit237, label %2421

2427:                                             ; preds = %2419
  %2428 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #86, !noalias !34535
  unreachable

2429:                                             ; preds = %2508, %2431, %2419
  %2430 = phi { ptr, i32 } [ %2420, %2419 ], [ %2432, %2431 ], [ %2509, %2508 ]
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %110) #89
  br label %566

2431:                                             ; preds = %.loopexit236, %2552, %2485, %.loopexit237
  %2432 = landingpad { ptr, i32 }
          cleanup
  br label %2429

2433:                                             ; preds = %.loopexit237
  call void @llvm.lifetime.end.p0(ptr nonnull %39), !noalias !34530
  br i1 %2395, label %2434, label %2437

2434:                                             ; preds = %2433
  %2435 = load i64, ptr %182, align 8, !noundef !1708
  %2436 = icmp ugt i64 %2396, %2435
  br i1 %2436, label %2485, label %2444, !prof !1803

2437:                                             ; preds = %2729, %2433
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %121, ptr noundef nonnull align 8 dereferenceable(24) %110, i64 24, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %110)
  call void @llvm.lifetime.end.p0(ptr nonnull %113)
  call void @llvm.lifetime.end.p0(ptr nonnull %114)
  call void @llvm.lifetime.end.p0(ptr nonnull %119)
  call void @llvm.experimental.noalias.scope.decl(metadata !34541)
  %2438 = load ptr, ptr %120, align 8, !alias.scope !34541, !noundef !1708
  %2439 = icmp eq ptr %2438, null
  br i1 %2439, label %2731, label %2440

2440:                                             ; preds = %2437
  %2441 = atomicrmw sub ptr %2438, i64 1 release, align 8, !noalias !34544
  %2442 = icmp eq i64 %2441, 1
  br i1 %2442, label %2443, label %2731

2443:                                             ; preds = %2440
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %120) #87
          to label %2731 unwind label %528, !inline_history !1744

2444:                                             ; preds = %2434
  %2445 = load ptr, ptr %551, align 8, !nonnull !1708, !noundef !1708
  call void @llvm.lifetime.start.p0(ptr nonnull %106)
  call void @llvm.experimental.noalias.scope.decl(metadata !34547)
  %2446 = load ptr, ptr %555, align 8, !noalias !34547, !noundef !1708
  %2447 = icmp eq ptr %2446, null
  br i1 %2447, label %2471, label %2448

2448:                                             ; preds = %2444
  %2449 = getelementptr inbounds nuw i8, ptr %2446, i64 16
  %2450 = load i64, ptr %2449, align 8
  %2451 = icmp ne i64 %2450, -1
  %2452 = getelementptr inbounds nuw i8, ptr %2446, i64 40
  %2453 = load i64, ptr %2452, align 8
  %2454 = icmp ne i64 %2453, -1
  %2455 = getelementptr inbounds nuw i8, ptr %2446, i64 336
  %2456 = load ptr, ptr %2455, align 8, !noundef !1708
  %2457 = icmp ne ptr %2456, null
  %2458 = select i1 %2457, i1 true, i1 %2451
  %2459 = select i1 %2458, i1 true, i1 %2454
  %2460 = getelementptr inbounds nuw i8, ptr %106, i64 192
  %2461 = getelementptr inbounds nuw i8, ptr %106, i64 160
  %2462 = getelementptr inbounds nuw i8, ptr %106, i64 184
  %2463 = getelementptr inbounds nuw i8, ptr %106, i64 8
  %2464 = getelementptr inbounds nuw i8, ptr %106, i64 16
  %2465 = getelementptr inbounds nuw i8, ptr %106, i64 32
  %2466 = getelementptr inbounds nuw i8, ptr %106, i64 40
  %2467 = getelementptr inbounds nuw i8, ptr %106, i64 56
  %2468 = getelementptr inbounds nuw i8, ptr %106, i64 64
  %2469 = getelementptr inbounds nuw i8, ptr %106, i64 72
  %2470 = getelementptr inbounds nuw i8, ptr %106, i64 88
  br i1 %2459, label %2484, label %2483

2471:                                             ; preds = %2444
  %2472 = getelementptr inbounds nuw i8, ptr %106, i64 192
  %2473 = getelementptr inbounds nuw i8, ptr %106, i64 160
  %2474 = getelementptr inbounds nuw i8, ptr %106, i64 184
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %2473, i8 0, i64 24, i1 false), !alias.scope !34547
  store i64 -1, ptr %2474, align 8, !alias.scope !34547
  store <4 x i8> <i8 0, i8 0, i8 0, i8 4>, ptr %2472, align 8, !alias.scope !34547
  store i64 0, ptr %106, align 8, !alias.scope !34547
  %2475 = getelementptr inbounds nuw i8, ptr %106, i64 8
  store ptr inttoptr (i64 8 to ptr), ptr %2475, align 8, !alias.scope !34547
  %2476 = getelementptr inbounds nuw i8, ptr %106, i64 16
  %2477 = getelementptr inbounds nuw i8, ptr %106, i64 32
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %2476, i8 0, i64 16, i1 false), !alias.scope !34547
  store ptr inttoptr (i64 8 to ptr), ptr %2477, align 8, !alias.scope !34547
  %2478 = getelementptr inbounds nuw i8, ptr %106, i64 40
  %2479 = getelementptr inbounds nuw i8, ptr %106, i64 56
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %2478, i8 0, i64 16, i1 false), !alias.scope !34547
  store ptr inttoptr (i64 8 to ptr), ptr %2479, align 8, !alias.scope !34547
  %2480 = getelementptr inbounds nuw i8, ptr %106, i64 64
  store i64 0, ptr %2480, align 8, !alias.scope !34547
  %2481 = getelementptr inbounds nuw i8, ptr %106, i64 72
  %2482 = getelementptr inbounds nuw i8, ptr %106, i64 88
  br label %2486

2483:                                             ; preds = %2448
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %2461, i8 0, i64 24, i1 false), !alias.scope !34547
  store i64 -1, ptr %2462, align 8, !alias.scope !34547
  store <4 x i8> <i8 0, i8 0, i8 0, i8 4>, ptr %2460, align 8, !alias.scope !34547
  store i64 0, ptr %106, align 8, !alias.scope !34547
  store ptr inttoptr (i64 8 to ptr), ptr %2463, align 8, !alias.scope !34547
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %2464, i8 0, i64 16, i1 false), !alias.scope !34547
  store ptr inttoptr (i64 8 to ptr), ptr %2465, align 8, !alias.scope !34547
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %2466, i8 0, i64 16, i1 false), !alias.scope !34547
  store ptr inttoptr (i64 8 to ptr), ptr %2467, align 8, !alias.scope !34547
  store i64 0, ptr %2468, align 8, !alias.scope !34547
  br label %2486

2484:                                             ; preds = %2448
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %2461, i8 0, i64 24, i1 false), !alias.scope !34547
  store i64 -1, ptr %2462, align 8, !alias.scope !34547
  store <4 x i8> <i8 0, i8 0, i8 1, i8 4>, ptr %2460, align 8, !alias.scope !34547
  store i64 0, ptr %106, align 8, !alias.scope !34547
  store ptr inttoptr (i64 8 to ptr), ptr %2463, align 8, !alias.scope !34547
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %2464, i8 0, i64 16, i1 false), !alias.scope !34547
  store ptr inttoptr (i64 8 to ptr), ptr %2465, align 8, !alias.scope !34547
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %2466, i8 0, i64 16, i1 false), !alias.scope !34547
  store ptr inttoptr (i64 8 to ptr), ptr %2467, align 8, !alias.scope !34547
  store i64 0, ptr %2468, align 8, !alias.scope !34547
  br label %2486

2485:                                             ; preds = %2434
; invoke core::slice::index::slice_index_fail
  invoke void @core::slice::index::slice_index_fail(i64 noundef %2396, i64 noundef %2435, i64 noundef %2435, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.a12f493ba210922c94e5446ac885c35e.391) #90
          to label %2730 unwind label %2431

2486:                                             ; preds = %2484, %2483, %2471
  %2487 = phi ptr [ %2469, %2484 ], [ %2469, %2483 ], [ %2481, %2471 ]
  %2488 = phi ptr [ %2470, %2484 ], [ %2470, %2483 ], [ %2482, %2471 ]
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %2487, i8 -1, i64 16, i1 false), !alias.scope !34547
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(65) %2488, i8 0, i64 65, i1 false), !alias.scope !34547
  %2489 = getelementptr inbounds nuw [40 x i8], ptr %2445, i64 %2435
  %2490 = icmp samesign eq i64 %2396, %2435
  br i1 %2490, label %.loopexit236, label %2491

2491:                                             ; preds = %2486
  %2492 = getelementptr inbounds nuw [40 x i8], ptr %2445, i64 %2396
  %2493 = getelementptr inbounds nuw i8, ptr %37, i64 8
  %2494 = getelementptr inbounds nuw i8, ptr %37, i64 16
  %2495 = getelementptr inbounds nuw i8, ptr %37, i64 24
  %2496 = getelementptr inbounds nuw i8, ptr %110, i64 16
  %2497 = getelementptr inbounds nuw i8, ptr %110, i64 8
  %2498 = getelementptr inbounds nuw i8, ptr %7, i64 8
  %2499 = getelementptr inbounds nuw i8, ptr %7, i64 12
  %2500 = getelementptr inbounds nuw i8, ptr %6, i64 8
  %wide.gep1906 = getelementptr inbounds nuw [8 x i8], ptr %2493, <4 x i64> <i64 0, i64 1, i64 2, i64 3>
  %wide.gep1907 = getelementptr inbounds nuw i8, <4 x ptr> %wide.gep1906, i64 4
  br label %2501

2501:                                             ; preds = %2704, %2491
  %2502 = phi ptr [ %2492, %2491 ], [ %2503, %2704 ]
  %2503 = getelementptr inbounds nuw i8, ptr %2502, i64 40
  call void @llvm.lifetime.start.p0(ptr nonnull %105)
; invoke <purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::pass::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::pass::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %105, ptr noalias nofree noundef align 8 dereferenceable(200) %106, ptr noundef nonnull align 16 %4)
          to label %2510 unwind label %2504

2504:                                             ; preds = %2541, %2523, %2501
  %2505 = landingpad { ptr, i32 }
          cleanup
  br label %2508

2506:                                             ; preds = %2595
  %2507 = landingpad { ptr, i32 }
          cleanup
  br label %2508

2508:                                             ; preds = %2717, %2714, %2506, %2504
  %2509 = phi { ptr, i32 } [ %2715, %2714 ], [ %2715, %2717 ], [ %2505, %2504 ], [ %2507, %2506 ]
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(200) %106)
          to label %2429 unwind label %545

2510:                                             ; preds = %2501
  %2511 = load i8, ptr %105, align 8, !range !1906, !noundef !1708
  %2512 = icmp eq i8 %2511, -1
  call void @llvm.lifetime.end.p0(ptr nonnull %105)
  br i1 %2512, label %2513, label %.loopexit236

2513:                                             ; preds = %2510
  call void @llvm.lifetime.start.p0(ptr nonnull %104)
  %2514 = load i64, ptr %2502, align 8, !range !1940, !noundef !1708
  %2515 = add i64 %2514, -1
  %2516 = icmp ugt i64 %2515, 4
  %2517 = getelementptr inbounds nuw i8, ptr %2502, i64 8
  br i1 %2516, label %2518, label %2523

2518:                                             ; preds = %2513
  %2519 = load ptr, ptr %2517, align 8, !nonnull !1708, !noundef !1708
  %2520 = getelementptr inbounds nuw i8, ptr %2502, i64 16
  %2521 = load i64, ptr %2520, align 8, !noundef !1708
  %2522 = add i64 %2521, -1
  br label %2523

2523:                                             ; preds = %2518, %2513
  %2524 = phi i64 [ %2522, %2518 ], [ %2515, %2513 ]
  %2525 = phi ptr [ %2519, %2518 ], [ %2517, %2513 ]
  %2526 = load ptr, ptr %124, align 8, !nonnull !1708, !noundef !1708
  %2527 = getelementptr inbounds nuw i8, ptr %2526, i64 16
  call void @llvm.lifetime.start.p0(ptr nonnull %7), !noalias !34550
; invoke <purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::term::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::term::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef align 16 captures(none) dereferenceable(96) %7, ptr noalias nofree noundef nonnull align 8 dereferenceable(216) %122, ptr noalias nofree noundef nonnull readonly align 4 captures(address, read_provenance) %2525, i64 noundef range(i64 0, 1152921504606846976) %2524, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(56) %2527, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %4)
          to label %2528 unwind label %2504, !inline_history !33933

2528:                                             ; preds = %2523
  %2529 = load i64, ptr %7, align 16, !range !2530, !noalias !34550, !noundef !1708
  %2530 = icmp eq i64 %2529, -1
  %2531 = load i32, ptr %2498, align 8, !noalias !34550
  %2532 = load i32, ptr %2499, align 4, !noalias !34550
  br i1 %2530, label %2538, label %2533

2533:                                             ; preds = %2528
  %2534 = getelementptr inbounds nuw i8, ptr %7, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(80) %104, ptr noundef nonnull align 16 dereferenceable(80) %2534, i64 80, i1 false), !noalias !34557
  call void @llvm.lifetime.end.p0(ptr nonnull %7), !noalias !34550
  %2535 = trunc i32 %2531 to i8
  %2536 = lshr i32 %2531, 8
  %2537 = trunc nuw i32 %2536 to i24
  br label %2552

2538:                                             ; preds = %2528
  call void @llvm.lifetime.end.p0(ptr nonnull %7), !noalias !34550
  %2539 = icmp eq i32 %2531, 2
  br i1 %2539, label %2540, label %2541

2540:                                             ; preds = %2538
  call void @llvm.lifetime.end.p0(ptr nonnull %104)
  br label %2704

2541:                                             ; preds = %2538
  call void @llvm.lifetime.start.p0(ptr nonnull %6), !noalias !34550
; invoke purrdf_sparql_eval::expr::ebv_term::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @purrdf_sparql_eval::expr::ebv_term::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef align 16 captures(none) dereferenceable(96) %6, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %4, i32 noundef %2531, i32 noundef %2532)
          to label %2542 unwind label %2504, !inline_history !33933

2542:                                             ; preds = %2541
  %2543 = load i64, ptr %6, align 16, !range !2530, !noalias !34550, !noundef !1708
  %2544 = icmp eq i64 %2543, -1
  %2545 = load i8, ptr %2500, align 8, !noalias !34550
  br i1 %2544, label %2562, label %2546

2546:                                             ; preds = %2542
  %2547 = getelementptr inbounds nuw i8, ptr %6, i64 9
  %2548 = load i24, ptr %2547, align 1, !noalias !34557
  %2549 = getelementptr inbounds nuw i8, ptr %6, i64 12
  %2550 = load i32, ptr %2549, align 4, !noalias !34557
  %2551 = getelementptr inbounds nuw i8, ptr %6, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(80) %104, ptr noundef nonnull align 16 dereferenceable(80) %2551, i64 80, i1 false), !noalias !34557
  call void @llvm.lifetime.end.p0(ptr nonnull %6), !noalias !34550
  br label %2552

2552:                                             ; preds = %2546, %2533
  %2553 = phi i24 [ %2537, %2533 ], [ %2548, %2546 ]
  %2554 = phi i8 [ %2535, %2533 ], [ %2545, %2546 ]
  %2555 = phi i32 [ %2532, %2533 ], [ %2550, %2546 ]
  %2556 = phi i64 [ %2529, %2533 ], [ %2543, %2546 ]
  %2557 = getelementptr inbounds nuw i8, ptr %0, i64 25
  store i24 %2553, ptr %2557, align 1
  %2558 = getelementptr inbounds nuw i8, ptr %0, i64 28
  store i32 %2555, ptr %2558, align 4
  %2559 = getelementptr inbounds nuw i8, ptr %0, i64 32
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(80) %2559, ptr noundef nonnull align 16 dereferenceable(80) %104, i64 80, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %104)
  %2560 = getelementptr inbounds nuw i8, ptr %0, i64 16
  store i64 %2556, ptr %2560, align 16
  %2561 = getelementptr inbounds nuw i8, ptr %0, i64 24
  store i8 %2554, ptr %2561, align 8
  store i64 1, ptr %0, align 16
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(200) %106)
          to label %2727 unwind label %2431

2562:                                             ; preds = %2542
  call void @llvm.lifetime.end.p0(ptr nonnull %6), !noalias !34550
  call void @llvm.lifetime.end.p0(ptr nonnull %104)
  %2563 = and i8 %2545, 1
  %2564 = icmp eq i8 %2563, 0
  br i1 %2564, label %2704, label %2565

2565:                                             ; preds = %2562
  call void @llvm.lifetime.start.p0(ptr nonnull %103)
  call void @llvm.experimental.noalias.scope.decl(metadata !34558)
  %2566 = load i64, ptr %2502, align 8, !range !1940, !alias.scope !34558, !noalias !34561, !noundef !1708
  %2567 = add i64 %2566, -1
  %2568 = icmp ugt i64 %2567, 4
  %2569 = getelementptr inbounds nuw i8, ptr %2502, i64 16
  %2570 = load i64, ptr %2569, align 8, !alias.scope !34558, !noalias !34561
  %2571 = add i64 %2570, -1
  %2572 = select i1 %2568, i64 %2571, i64 %2567
  %2573 = icmp ugt i64 %2572, 4
  br i1 %2573, label %2583, label %2574

2574:                                             ; preds = %2565
  call void @llvm.lifetime.start.p0(ptr nonnull %37), !noalias !34563
  %2575 = icmp ugt i64 %2566, 5
  %2576 = load ptr, ptr %2517, align 8, !alias.scope !34558, !noalias !34561, !nonnull !1708
  %2577 = select i1 %2575, ptr %2576, ptr %2517
  %2578 = icmp eq i64 %2572, 0
  br i1 %2578, label %2591, label %vector.body1899

vector.body1899:                                  ; preds = %2574
  %trip.count.minus.1 = add nsw i64 %2572, -1
  %broadcast.splatinsert1897 = insertelement <4 x i64> poison, i64 %trip.count.minus.1, i64 0
  %broadcast.splat1898 = shufflevector <4 x i64> %broadcast.splatinsert1897, <4 x i64> poison, <4 x i32> zeroinitializer
  %2579 = icmp uge <4 x i64> %broadcast.splat1898, <i64 0, i64 1, i64 2, i64 3>
  %wide.gep1902 = getelementptr inbounds nuw [8 x i8], ptr %2577, <4 x i64> <i64 0, i64 1, i64 2, i64 3>
  %wide.masked.gather1903 = call <4 x i32> @llvm.masked.gather.v4i32.v4p0(<4 x ptr> align 4 %wide.gep1902, <4 x i1> %2579, <4 x i32> poison), !noalias !34561
  %wide.gep1904 = getelementptr i8, <4 x ptr> %wide.gep1902, i64 4
  %wide.masked.gather1905 = call <4 x i32> @llvm.masked.gather.v4i32.v4p0(<4 x ptr> align 4 %wide.gep1904, <4 x i1> %2579, <4 x i32> poison), !noalias !34561
  %2580 = icmp eq <4 x i32> %wide.masked.gather1903, splat (i32 2)
  %2581 = select <4 x i1> %2580, <4 x i32> undef, <4 x i32> %wide.masked.gather1905
  call void @llvm.masked.scatter.v4i32.v4p0(<4 x i32> %wide.masked.gather1903, <4 x ptr> align 4 %wide.gep1906, <4 x i1> %2579), !noalias !34563
  call void @llvm.masked.scatter.v4i32.v4p0(<4 x i32> %2581, <4 x ptr> align 4 %wide.gep1907, <4 x i1> %2579), !noalias !34563
  %2582 = add nuw nsw i64 %2572, 1
  br label %2591

2583:                                             ; preds = %2565
  %2584 = shl i64 %2572, 3
  %2585 = icmp ugt i64 %2572, 2305843009213693951
  %2586 = icmp ugt i64 %2584, 9223372036854775804
  %2587 = or i1 %2585, %2586
  br i1 %2587, label %2595, label %2588, !prof !5895

2588:                                             ; preds = %2583
; call __rustc::__rust_alloc
  %2589 = call noundef align 4 ptr @__rustc::__rust_alloc(i64 noundef %2584, i64 noundef range(i64 1, -9223372036854775807) 4) #88, !noalias !34564
  %2590 = icmp eq ptr %2589, null
  br i1 %2590, label %2595, label %iter.check1876

2591:                                             ; preds = %vector.body1899, %2574
  %2592 = phi i64 [ 1, %2574 ], [ %2582, %vector.body1899 ]
  store i64 %2592, ptr %37, align 8, !noalias !34563
  %2593 = load ptr, ptr %2493, align 8, !noalias !34558
  %2594 = load i64, ptr %2494, align 8, !noalias !34558
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %103, ptr noundef nonnull align 8 dereferenceable(16) %2495, i64 16, i1 false), !noalias !34558
  call void @llvm.lifetime.end.p0(ptr nonnull %37), !noalias !34563
  br label %2706

2595:                                             ; preds = %2588, %2583
  %2596 = phi i64 [ 4, %2588 ], [ 0, %2583 ]
; invoke alloc::raw_vec::handle_error
  invoke void @alloc::raw_vec::handle_error(i64 noundef %2596, i64 %2584) #90
          to label %2597 unwind label %2506

2597:                                             ; preds = %2595
  unreachable

iter.check1876:                                   ; preds = %2588
  %2598 = load ptr, ptr %2517, align 8, !alias.scope !34558, !noalias !34561, !nonnull !1708
  %2599 = select i1 %2568, ptr %2598, ptr %2517
  %min.iters.check1859 = icmp ult i64 %2572, 8
  br i1 %min.iters.check1859, label %vec.epilog.scalar.ph1877.preheader, label %vector.memcheck

vector.memcheck:                                  ; preds = %iter.check1876
  %scevgep = getelementptr i8, ptr %2589, i64 %2584
  %scevgep1858 = getelementptr i8, ptr %2599, i64 %2584
  %bound0 = icmp ult ptr %2589, %scevgep1858
  %bound1 = icmp ult ptr %2599, %scevgep
  %found.conflict = and i1 %bound0, %bound1
  br i1 %found.conflict, label %vec.epilog.scalar.ph1877.preheader, label %vector.main.loop.iter.check1860

vector.main.loop.iter.check1860:                  ; preds = %vector.memcheck
  %min.iters.check1861 = icmp ult i64 %2572, 32
  br i1 %min.iters.check1861, label %vec.epilog.ph1880, label %vector.ph1862

vector.ph1862:                                    ; preds = %vector.main.loop.iter.check1860
  %n.mod.vf1863 = and i64 %2572, 24
  %n.vec1864 = and i64 %2572, 2305843009213693920
  br label %vector.body1865

vector.body1865:                                  ; preds = %vector.body1865, %vector.ph1862
  %index1866 = phi i64 [ 0, %vector.ph1862 ], [ %index.next1872, %vector.body1865 ]
  %2600 = or disjoint i64 %index1866, 16
  %2601 = getelementptr inbounds nuw [8 x i8], ptr %2599, i64 %index1866
  %2602 = getelementptr inbounds nuw [8 x i8], ptr %2599, i64 %2600
  %wide.vec = load <32 x i32>, ptr %2601, align 4, !alias.scope !34567, !noalias !34570
  %strided.vec = shufflevector <32 x i32> %wide.vec, <32 x i32> poison, <16 x i32> <i32 0, i32 2, i32 4, i32 6, i32 8, i32 10, i32 12, i32 14, i32 16, i32 18, i32 20, i32 22, i32 24, i32 26, i32 28, i32 30>
  %strided.vec1867 = shufflevector <32 x i32> %wide.vec, <32 x i32> poison, <16 x i32> <i32 1, i32 3, i32 5, i32 7, i32 9, i32 11, i32 13, i32 15, i32 17, i32 19, i32 21, i32 23, i32 25, i32 27, i32 29, i32 31>
  %wide.vec1868 = load <32 x i32>, ptr %2602, align 4, !alias.scope !34567, !noalias !34570
  %strided.vec1869 = shufflevector <32 x i32> %wide.vec1868, <32 x i32> poison, <16 x i32> <i32 0, i32 2, i32 4, i32 6, i32 8, i32 10, i32 12, i32 14, i32 16, i32 18, i32 20, i32 22, i32 24, i32 26, i32 28, i32 30>
  %strided.vec1870 = shufflevector <32 x i32> %wide.vec1868, <32 x i32> poison, <16 x i32> <i32 1, i32 3, i32 5, i32 7, i32 9, i32 11, i32 13, i32 15, i32 17, i32 19, i32 21, i32 23, i32 25, i32 27, i32 29, i32 31>
  %2603 = icmp eq <16 x i32> %strided.vec, splat (i32 2)
  %2604 = icmp eq <16 x i32> %strided.vec1869, splat (i32 2)
  %2605 = select <16 x i1> %2603, <16 x i32> undef, <16 x i32> %strided.vec1867
  %2606 = select <16 x i1> %2604, <16 x i32> undef, <16 x i32> %strided.vec1870
  %2607 = getelementptr inbounds nuw [8 x i8], ptr %2589, i64 %index1866
  %2608 = getelementptr inbounds nuw [8 x i8], ptr %2589, i64 %2600
  %interleaved.vec = shufflevector <16 x i32> %strided.vec, <16 x i32> %2605, <32 x i32> <i32 0, i32 16, i32 1, i32 17, i32 2, i32 18, i32 3, i32 19, i32 4, i32 20, i32 5, i32 21, i32 6, i32 22, i32 7, i32 23, i32 8, i32 24, i32 9, i32 25, i32 10, i32 26, i32 11, i32 27, i32 12, i32 28, i32 13, i32 29, i32 14, i32 30, i32 15, i32 31>
  store <32 x i32> %interleaved.vec, ptr %2607, align 4, !alias.scope !34583, !noalias !34585
  %interleaved.vec1871 = shufflevector <16 x i32> %strided.vec1869, <16 x i32> %2606, <32 x i32> <i32 0, i32 16, i32 1, i32 17, i32 2, i32 18, i32 3, i32 19, i32 4, i32 20, i32 5, i32 21, i32 6, i32 22, i32 7, i32 23, i32 8, i32 24, i32 9, i32 25, i32 10, i32 26, i32 11, i32 27, i32 12, i32 28, i32 13, i32 29, i32 14, i32 30, i32 15, i32 31>
  store <32 x i32> %interleaved.vec1871, ptr %2608, align 4, !alias.scope !34583, !noalias !34585
  %index.next1872 = add nuw i64 %index1866, 32
  %2609 = icmp eq i64 %index.next1872, %n.vec1864
  br i1 %2609, label %middle.block1873, label %vector.body1865, !llvm.loop !34592

middle.block1873:                                 ; preds = %vector.body1865
  %ind.escape = add nsw i64 %n.vec1864, -1
  %cmp.n1874 = icmp eq i64 %2572, %n.vec1864
  br i1 %cmp.n1874, label %.loopexit1911, label %vec.epilog.iter.check1878

vec.epilog.iter.check1878:                        ; preds = %middle.block1873
  %min.epilog.iters.check1879 = icmp eq i64 %n.mod.vf1863, 0
  br i1 %min.epilog.iters.check1879, label %vec.epilog.scalar.ph1877.preheader, label %vec.epilog.ph1880, !prof !29315

vec.epilog.ph1880:                                ; preds = %vector.main.loop.iter.check1860, %vec.epilog.iter.check1878
  %vec.epilog.resume.val1875 = phi i64 [ %n.vec1864, %vec.epilog.iter.check1878 ], [ 0, %vector.main.loop.iter.check1860 ]
  %n.vec1882 = and i64 %2572, 2305843009213693944
  br label %vec.epilog.vector.body1883

vec.epilog.vector.body1883:                       ; preds = %vec.epilog.vector.body1883, %vec.epilog.ph1880
  %index1884 = phi i64 [ %vec.epilog.resume.val1875, %vec.epilog.ph1880 ], [ %index.next1889, %vec.epilog.vector.body1883 ]
  %2610 = getelementptr inbounds nuw [8 x i8], ptr %2599, i64 %index1884
  %wide.vec1885 = load <16 x i32>, ptr %2610, align 4, !alias.scope !34567, !noalias !34570
  %strided.vec1886 = shufflevector <16 x i32> %wide.vec1885, <16 x i32> poison, <8 x i32> <i32 0, i32 2, i32 4, i32 6, i32 8, i32 10, i32 12, i32 14>
  %strided.vec1887 = shufflevector <16 x i32> %wide.vec1885, <16 x i32> poison, <8 x i32> <i32 1, i32 3, i32 5, i32 7, i32 9, i32 11, i32 13, i32 15>
  %2611 = icmp eq <8 x i32> %strided.vec1886, splat (i32 2)
  %2612 = select <8 x i1> %2611, <8 x i32> undef, <8 x i32> %strided.vec1887
  %2613 = getelementptr inbounds nuw [8 x i8], ptr %2589, i64 %index1884
  %interleaved.vec1888 = shufflevector <8 x i32> %strided.vec1886, <8 x i32> %2612, <16 x i32> <i32 0, i32 8, i32 1, i32 9, i32 2, i32 10, i32 3, i32 11, i32 4, i32 12, i32 5, i32 13, i32 6, i32 14, i32 7, i32 15>
  store <16 x i32> %interleaved.vec1888, ptr %2613, align 4, !alias.scope !34583, !noalias !34585
  %index.next1889 = add nuw i64 %index1884, 8
  %2614 = icmp eq i64 %index.next1889, %n.vec1882
  br i1 %2614, label %vec.epilog.middle.block1890, label %vec.epilog.vector.body1883, !llvm.loop !34593

vec.epilog.middle.block1890:                      ; preds = %vec.epilog.vector.body1883
  %ind.escape1891 = add nsw i64 %n.vec1882, -1
  %cmp.n1892 = icmp eq i64 %2572, %n.vec1882
  br i1 %cmp.n1892, label %.loopexit1911, label %vec.epilog.scalar.ph1877.preheader

vec.epilog.scalar.ph1877.preheader:               ; preds = %vector.memcheck, %iter.check1876, %vec.epilog.iter.check1878, %vec.epilog.middle.block1890
  %.ph = phi i64 [ 0, %iter.check1876 ], [ 0, %vector.memcheck ], [ %n.vec1864, %vec.epilog.iter.check1878 ], [ %n.vec1882, %vec.epilog.middle.block1890 ]
  %xtraiter2277 = and i64 %2572, 7
  %lcmp.mod2278.not = icmp eq i64 %xtraiter2277, 0
  br i1 %lcmp.mod2278.not, label %vec.epilog.scalar.ph1877.prol.loopexit, label %vec.epilog.scalar.ph1877.prol

vec.epilog.scalar.ph1877.prol:                    ; preds = %vec.epilog.scalar.ph1877.preheader, %vec.epilog.scalar.ph1877.prol
  %2615 = phi i64 [ %2624, %vec.epilog.scalar.ph1877.prol ], [ %.ph, %vec.epilog.scalar.ph1877.preheader ]
  %prol.iter2279 = phi i64 [ %prol.iter2279.next, %vec.epilog.scalar.ph1877.prol ], [ 0, %vec.epilog.scalar.ph1877.preheader ]
  %2616 = getelementptr inbounds nuw [8 x i8], ptr %2599, i64 %2615
  %2617 = load i32, ptr %2616, align 4, !range !1947, !noalias !34570, !noundef !1708
  %2618 = getelementptr i8, ptr %2616, i64 4
  %2619 = load i32, ptr %2618, align 4, !noalias !34570
  %2620 = icmp eq i32 %2617, 2
  %2621 = select i1 %2620, i32 undef, i32 %2619
  %2622 = getelementptr inbounds nuw [8 x i8], ptr %2589, i64 %2615
  store i32 %2617, ptr %2622, align 4, !noalias !34585
  %2623 = getelementptr inbounds nuw i8, ptr %2622, i64 4
  store i32 %2621, ptr %2623, align 4, !noalias !34585
  %2624 = add nuw nsw i64 %2615, 1
  %prol.iter2279.next = add i64 %prol.iter2279, 1
  %prol.iter2279.cmp.not = icmp eq i64 %prol.iter2279.next, %xtraiter2277
  br i1 %prol.iter2279.cmp.not, label %vec.epilog.scalar.ph1877.prol.loopexit, label %vec.epilog.scalar.ph1877.prol, !llvm.loop !34594

vec.epilog.scalar.ph1877.prol.loopexit:           ; preds = %vec.epilog.scalar.ph1877.prol, %vec.epilog.scalar.ph1877.preheader
  %.lcssa1922.unr = phi i64 [ poison, %vec.epilog.scalar.ph1877.preheader ], [ %2615, %vec.epilog.scalar.ph1877.prol ]
  %.unr2280 = phi i64 [ %.ph, %vec.epilog.scalar.ph1877.preheader ], [ %2624, %vec.epilog.scalar.ph1877.prol ]
  %2625 = sub nsw i64 %.ph, %2572
  %2626 = icmp ugt i64 %2625, -8
  br i1 %2626, label %.loopexit1911, label %vec.epilog.scalar.ph1877

vec.epilog.scalar.ph1877:                         ; preds = %vec.epilog.scalar.ph1877.prol.loopexit, %vec.epilog.scalar.ph1877
  %2627 = phi i64 [ %2699, %vec.epilog.scalar.ph1877 ], [ %.unr2280, %vec.epilog.scalar.ph1877.prol.loopexit ]
  %2628 = getelementptr inbounds nuw [8 x i8], ptr %2599, i64 %2627
  %2629 = load i32, ptr %2628, align 4, !range !1947, !noalias !34570, !noundef !1708
  %2630 = getelementptr i8, ptr %2628, i64 4
  %2631 = load i32, ptr %2630, align 4, !noalias !34570
  %2632 = icmp eq i32 %2629, 2
  %2633 = select i1 %2632, i32 undef, i32 %2631
  %2634 = getelementptr inbounds nuw [8 x i8], ptr %2589, i64 %2627
  store i32 %2629, ptr %2634, align 4, !noalias !34585
  %2635 = getelementptr inbounds nuw i8, ptr %2634, i64 4
  store i32 %2633, ptr %2635, align 4, !noalias !34585
  %2636 = add nuw nsw i64 %2627, 1
  %2637 = getelementptr inbounds nuw [8 x i8], ptr %2599, i64 %2636
  %2638 = load i32, ptr %2637, align 4, !range !1947, !noalias !34570, !noundef !1708
  %2639 = getelementptr i8, ptr %2637, i64 4
  %2640 = load i32, ptr %2639, align 4, !noalias !34570
  %2641 = icmp eq i32 %2638, 2
  %2642 = select i1 %2641, i32 undef, i32 %2640
  %2643 = getelementptr inbounds nuw [8 x i8], ptr %2589, i64 %2636
  store i32 %2638, ptr %2643, align 4, !noalias !34585
  %2644 = getelementptr inbounds nuw i8, ptr %2643, i64 4
  store i32 %2642, ptr %2644, align 4, !noalias !34585
  %2645 = add nuw nsw i64 %2627, 2
  %2646 = getelementptr inbounds nuw [8 x i8], ptr %2599, i64 %2645
  %2647 = load i32, ptr %2646, align 4, !range !1947, !noalias !34570, !noundef !1708
  %2648 = getelementptr i8, ptr %2646, i64 4
  %2649 = load i32, ptr %2648, align 4, !noalias !34570
  %2650 = icmp eq i32 %2647, 2
  %2651 = select i1 %2650, i32 undef, i32 %2649
  %2652 = getelementptr inbounds nuw [8 x i8], ptr %2589, i64 %2645
  store i32 %2647, ptr %2652, align 4, !noalias !34585
  %2653 = getelementptr inbounds nuw i8, ptr %2652, i64 4
  store i32 %2651, ptr %2653, align 4, !noalias !34585
  %2654 = add nuw nsw i64 %2627, 3
  %2655 = getelementptr inbounds nuw [8 x i8], ptr %2599, i64 %2654
  %2656 = load i32, ptr %2655, align 4, !range !1947, !noalias !34570, !noundef !1708
  %2657 = getelementptr i8, ptr %2655, i64 4
  %2658 = load i32, ptr %2657, align 4, !noalias !34570
  %2659 = icmp eq i32 %2656, 2
  %2660 = select i1 %2659, i32 undef, i32 %2658
  %2661 = getelementptr inbounds nuw [8 x i8], ptr %2589, i64 %2654
  store i32 %2656, ptr %2661, align 4, !noalias !34585
  %2662 = getelementptr inbounds nuw i8, ptr %2661, i64 4
  store i32 %2660, ptr %2662, align 4, !noalias !34585
  %2663 = add nuw nsw i64 %2627, 4
  %2664 = getelementptr inbounds nuw [8 x i8], ptr %2599, i64 %2663
  %2665 = load i32, ptr %2664, align 4, !range !1947, !noalias !34570, !noundef !1708
  %2666 = getelementptr i8, ptr %2664, i64 4
  %2667 = load i32, ptr %2666, align 4, !noalias !34570
  %2668 = icmp eq i32 %2665, 2
  %2669 = select i1 %2668, i32 undef, i32 %2667
  %2670 = getelementptr inbounds nuw [8 x i8], ptr %2589, i64 %2663
  store i32 %2665, ptr %2670, align 4, !noalias !34585
  %2671 = getelementptr inbounds nuw i8, ptr %2670, i64 4
  store i32 %2669, ptr %2671, align 4, !noalias !34585
  %2672 = add nuw nsw i64 %2627, 5
  %2673 = getelementptr inbounds nuw [8 x i8], ptr %2599, i64 %2672
  %2674 = load i32, ptr %2673, align 4, !range !1947, !noalias !34570, !noundef !1708
  %2675 = getelementptr i8, ptr %2673, i64 4
  %2676 = load i32, ptr %2675, align 4, !noalias !34570
  %2677 = icmp eq i32 %2674, 2
  %2678 = select i1 %2677, i32 undef, i32 %2676
  %2679 = getelementptr inbounds nuw [8 x i8], ptr %2589, i64 %2672
  store i32 %2674, ptr %2679, align 4, !noalias !34585
  %2680 = getelementptr inbounds nuw i8, ptr %2679, i64 4
  store i32 %2678, ptr %2680, align 4, !noalias !34585
  %2681 = add nuw nsw i64 %2627, 6
  %2682 = getelementptr inbounds nuw [8 x i8], ptr %2599, i64 %2681
  %2683 = load i32, ptr %2682, align 4, !range !1947, !noalias !34570, !noundef !1708
  %2684 = getelementptr i8, ptr %2682, i64 4
  %2685 = load i32, ptr %2684, align 4, !noalias !34570
  %2686 = icmp eq i32 %2683, 2
  %2687 = select i1 %2686, i32 undef, i32 %2685
  %2688 = getelementptr inbounds nuw [8 x i8], ptr %2589, i64 %2681
  store i32 %2683, ptr %2688, align 4, !noalias !34585
  %2689 = getelementptr inbounds nuw i8, ptr %2688, i64 4
  store i32 %2687, ptr %2689, align 4, !noalias !34585
  %2690 = add nuw nsw i64 %2627, 7
  %2691 = getelementptr inbounds nuw [8 x i8], ptr %2599, i64 %2690
  %2692 = load i32, ptr %2691, align 4, !range !1947, !noalias !34570, !noundef !1708
  %2693 = getelementptr i8, ptr %2691, i64 4
  %2694 = load i32, ptr %2693, align 4, !noalias !34570
  %2695 = icmp eq i32 %2692, 2
  %2696 = select i1 %2695, i32 undef, i32 %2694
  %2697 = getelementptr inbounds nuw [8 x i8], ptr %2589, i64 %2690
  store i32 %2692, ptr %2697, align 4, !noalias !34585
  %2698 = getelementptr inbounds nuw i8, ptr %2697, i64 4
  store i32 %2696, ptr %2698, align 4, !noalias !34585
  %2699 = add nuw nsw i64 %2627, 8
  %2700 = icmp eq i64 %2699, %2572
  br i1 %2700, label %.loopexit1911, label %vec.epilog.scalar.ph1877, !llvm.loop !34595

.loopexit1911:                                    ; preds = %vec.epilog.scalar.ph1877.prol.loopexit, %vec.epilog.scalar.ph1877, %vec.epilog.middle.block1890, %middle.block1873
  %.lcssa = phi i64 [ %ind.escape1891, %vec.epilog.middle.block1890 ], [ %ind.escape, %middle.block1873 ], [ %.lcssa1922.unr, %vec.epilog.scalar.ph1877.prol.loopexit ], [ %2690, %vec.epilog.scalar.ph1877 ]
  %2701 = icmp samesign ult i64 %2572, 1152921504606846976
  call void @llvm.assume(i1 %2701)
  %2702 = add nuw nsw i64 %.lcssa, 2
  %2703 = add nuw nsw i64 %2572, 1
  br label %2706

2704:                                             ; preds = %2720, %2562, %2540
  %2705 = icmp eq ptr %2503, %2489
  br i1 %2705, label %.loopexit236, label %2501

2706:                                             ; preds = %.loopexit1911, %2591
  %2707 = phi ptr [ %2589, %.loopexit1911 ], [ %2593, %2591 ]
  %2708 = phi i64 [ %2703, %.loopexit1911 ], [ %2592, %2591 ]
  %2709 = phi i64 [ %2702, %.loopexit1911 ], [ %2594, %2591 ]
  call void @llvm.experimental.noalias.scope.decl(metadata !34596)
  %2710 = load i64, ptr %2496, align 8, !alias.scope !34596, !noalias !34599, !noundef !1708
  %2711 = load i64, ptr %110, align 8, !range !1817, !alias.scope !34596, !noalias !34599, !noundef !1708
  %2712 = icmp eq i64 %2710, %2711
  br i1 %2712, label %2713, label %2720

2713:                                             ; preds = %2706
; invoke <alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one
  invoke void @<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %110)
          to label %2720 unwind label %2714, !noalias !34599

2714:                                             ; preds = %2713
  %2715 = landingpad { ptr, i32 }
          cleanup
  %2716 = icmp samesign ugt i64 %2708, 5
  br i1 %2716, label %2717, label %2508

2717:                                             ; preds = %2714
  %2718 = shl nuw i64 %2708, 3
  %2719 = add i64 %2718, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %2707, i64 noundef %2719, i64 noundef range(i64 1, -9223372036854775807) 4) #88, !noalias !34601
  br label %2508

2720:                                             ; preds = %2713, %2706
  %2721 = load ptr, ptr %2497, align 8, !alias.scope !34596, !noalias !34599, !nonnull !1708, !noundef !1708
  %2722 = getelementptr inbounds nuw [40 x i8], ptr %2721, i64 %2710
  store i64 %2708, ptr %2722, align 8, !noalias !34596
  %2723 = getelementptr inbounds nuw i8, ptr %2722, i64 8
  store ptr %2707, ptr %2723, align 8, !noalias !34596
  %2724 = getelementptr inbounds nuw i8, ptr %2722, i64 16
  store i64 %2709, ptr %2724, align 8, !noalias !34596
  %2725 = getelementptr inbounds nuw i8, ptr %2722, i64 24
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %2725, ptr noundef nonnull align 8 dereferenceable(16) %103, i64 16, i1 false), !noalias !34596
  %2726 = add i64 %2710, 1
  store i64 %2726, ptr %2496, align 8, !alias.scope !34596, !noalias !34599
  call void @llvm.lifetime.end.p0(ptr nonnull %103)
  br label %2704

2727:                                             ; preds = %2552
  call void @llvm.lifetime.end.p0(ptr nonnull %106)
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %110)
  call void @llvm.lifetime.end.p0(ptr nonnull %110)
  call void @llvm.lifetime.end.p0(ptr nonnull %113)
  br label %2728

2728:                                             ; preds = %2727, %2384
  call void @llvm.lifetime.end.p0(ptr nonnull %114)
  call void @llvm.lifetime.end.p0(ptr nonnull %119)
  br label %2981

.loopexit236:                                     ; preds = %2704, %2510, %2486
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(200) %106)
          to label %2729 unwind label %2431

2729:                                             ; preds = %.loopexit236
  call void @llvm.lifetime.end.p0(ptr nonnull %106)
  br label %2437

2730:                                             ; preds = %2485
  unreachable

2731:                                             ; preds = %2443, %2440, %2437
  call void @llvm.lifetime.end.p0(ptr nonnull %120)
  br label %533

2732:                                             ; preds = %2797
  %2733 = landingpad { ptr, i32 }
          cleanup
  br label %524

2734:                                             ; preds = %533
  %2735 = getelementptr inbounds nuw i8, ptr %536, i64 16
  %2736 = load i8, ptr %2735, align 8, !noalias !33975
  %2737 = icmp eq i8 %2736, -1
  br i1 %2737, label %2744, label %2738

2738:                                             ; preds = %2734
  %2739 = getelementptr inbounds nuw i8, ptr %536, i64 17
  call void @llvm.lifetime.start.p0(ptr nonnull %95)
  store i8 %2736, ptr %95, align 8
  %2740 = getelementptr inbounds nuw i8, ptr %95, i64 1
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(23) %2740, ptr noundef nonnull align 1 dereferenceable(23) %2739, i64 23, i1 false)
  call void @llvm.lifetime.start.p0(ptr nonnull %94)
  %2741 = load ptr, ptr %124, align 8, !nonnull !1708, !noundef !1708
  %2742 = atomicrmw add ptr %2741, i64 1 monotonic, align 8
  %2743 = icmp slt i64 %2742, 0
  br i1 %2743, label %2800, label %2798

2744:                                             ; preds = %2734, %533
  call void @llvm.lifetime.start.p0(ptr nonnull %93)
  call void @llvm.lifetime.start.p0(ptr nonnull %92)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(104) %92, ptr noundef nonnull align 8 dereferenceable(104) %128, i64 104, i1 false)
  call void @llvm.lifetime.start.p0(ptr nonnull %91)
  %2745 = load ptr, ptr %124, align 8, !nonnull !1708, !noundef !1708
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %91, ptr noundef nonnull align 8 dereferenceable(24) %121, i64 24, i1 false)
  %2746 = getelementptr inbounds nuw i8, ptr %91, i64 24
  store ptr %2745, ptr %2746, align 8
  call void @llvm.experimental.noalias.scope.decl(metadata !34604)
  call void @llvm.experimental.noalias.scope.decl(metadata !34607)
  call void @llvm.experimental.noalias.scope.decl(metadata !34609)
  %2747 = load i64, ptr %92, align 8, !range !2062, !alias.scope !34607, !noalias !34611, !noundef !1708
  %2748 = icmp eq i64 %2747, -1
  br i1 %2748, label %2750, label %2749

2749:                                             ; preds = %2744
; call <purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
  call fastcc void @<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(96) %93, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(32) %91, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(104) %128)
  br label %2752

2750:                                             ; preds = %2744
  %2751 = getelementptr inbounds nuw i8, ptr %93, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %2751, ptr noundef nonnull readonly align 8 dereferenceable(32) %91, i64 32, i1 false), !alias.scope !34611, !noalias !34607
  store i64 -1, ptr %93, align 8, !alias.scope !34604, !noalias !34612
  br label %2752

2752:                                             ; preds = %2750, %2749
  %2753 = getelementptr inbounds nuw i8, ptr %92, i64 72
  %2754 = load i64, ptr %2753, align 8, !range !1940, !alias.scope !34613, !noalias !34611, !noundef !1708
  %2755 = icmp ugt i64 %2754, 5
  br i1 %2755, label %2756, label %2790

2756:                                             ; preds = %2752
  %2757 = getelementptr inbounds nuw i8, ptr %92, i64 80
  %2758 = load ptr, ptr %2757, align 8, !alias.scope !34607, !noalias !34611, !nonnull !1708, !noundef !1708
  %2759 = mul i64 %2754, 3
  %2760 = add i64 %2759, -3
  %2761 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %2762 = load i64, ptr %2761, align 8, !noalias !34616, !noundef !1708
  %2763 = call i64 @llvm.umin.i64(i64 %2760, i64 9223372036854775807)
  %2764 = call i64 @llvm.ssub.sat.i64(i64 %2762, i64 %2763)
  store i64 %2764, ptr %2761, align 8, !noalias !34616
  %2765 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %2766 = load i64, ptr %2765, align 8, !noalias !34616, !noundef !1708
  %2767 = icmp slt i64 %2764, %2766
  br i1 %2767, label %2768, label %.preheader1919

2768:                                             ; preds = %2756
  store i64 %2764, ptr %2765, align 8, !noalias !34616
  br label %.preheader1919

.preheader1919:                                   ; preds = %2768, %2756
  br label %2769

2769:                                             ; preds = %.preheader1919, %2772
  %2770 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952) acquire, align 8, !noalias !34616
  %2771 = icmp slt i64 %2770, 0
  br i1 %2771, label %2772, label %__rustc::__rust_dealloc (.exit222)

2772:                                             ; preds = %2769
  %2773 = add nsw i64 %2770, 1
  %2774 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 %2770, i64 %2773 acq_rel acquire, align 8, !noalias !34616
  %2775 = extractvalue { i64, i1 } %2774, 1
  br i1 %2775, label %2776, label %2769

2776:                                             ; preds = %2772
  %2777 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %2763 monotonic, align 8, !noalias !34616
  %2778 = call i64 @llvm.ssub.sat.i64(i64 %2777, i64 %2763)
  %2779 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !34616
  br label %2780

2780:                                             ; preds = %2783, %2776
  %2781 = phi i64 [ %2779, %2776 ], [ %2786, %2783 ]
  %2782 = icmp slt i64 %2778, %2781
  br i1 %2782, label %2783, label %2787

2783:                                             ; preds = %2780
  %2784 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %2781, i64 %2778 monotonic monotonic, align 8, !noalias !34616
  %2785 = extractvalue { i64, i1 } %2784, 1
  %2786 = extractvalue { i64, i1 } %2784, 0
  br i1 %2785, label %2787, label %2780

2787:                                             ; preds = %2783, %2780
  %2788 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 1 release, align 8, !noalias !34616
  br label %__rustc::__rust_dealloc (.exit222)

__rustc::__rust_dealloc (.exit222): ; preds = %2769, %2787
  %2789 = icmp ne i64 %2760, 0
  call void @llvm.assume(i1 %2789), !noalias !34616
  call void @free(ptr noundef nonnull %2758) #88, !noalias !34616
  br label %2790

2790:                                             ; preds = %__rustc::__rust_dealloc (.exit222), %2752
  %2791 = getelementptr inbounds nuw i8, ptr %92, i64 96
  call void @llvm.experimental.noalias.scope.decl(metadata !34619)
  %2792 = load ptr, ptr %2791, align 8, !alias.scope !34622, !noalias !34611, !noundef !1708
  %2793 = icmp eq ptr %2792, null
  br i1 %2793, label %2884, label %2794

2794:                                             ; preds = %2790
  %2795 = atomicrmw sub ptr %2792, i64 1 release, align 8, !noalias !34623
  %2796 = icmp eq i64 %2795, 1
  br i1 %2796, label %2797, label %2884

2797:                                             ; preds = %2794
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %2791) #87
          to label %2884 unwind label %2732

2798:                                             ; preds = %2738
  %2799 = load ptr, ptr %124, align 8, !nonnull !1708, !noundef !1708
; invoke <purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::barred_at
  invoke fastcc void @<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::barred_at(ptr noalias nofree noundef align 8 captures(address) dereferenceable(96) %94, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(144) %1, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %95, ptr noundef nonnull %2799)
          to label %2801 unwind label %2979

2800:                                             ; preds = %2738
  call void @llvm.trap()
  unreachable

2801:                                             ; preds = %2798
  %2802 = getelementptr inbounds nuw i8, ptr %0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(96) %2802, ptr noundef nonnull align 8 dereferenceable(96) %94, i64 96, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %94)
  store i64 0, ptr %0, align 16
  call void @llvm.lifetime.end.p0(ptr nonnull %95)
  call void @llvm.experimental.noalias.scope.decl(metadata !34628)
  %2803 = getelementptr inbounds nuw i8, ptr %121, i64 8
  %2804 = load ptr, ptr %2803, align 8, !alias.scope !34628, !nonnull !1708, !noundef !1708
  %2805 = getelementptr inbounds nuw i8, ptr %121, i64 16
  %2806 = load i64, ptr %2805, align 8, !alias.scope !34628, !noundef !1708
  call void @llvm.experimental.noalias.scope.decl(metadata !34631)
  %2807 = icmp eq i64 %2806, 0
  br i1 %2807, label %.loopexit234, label %.preheader233

.preheader233:                                    ; preds = %2801
  %2808 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %2809 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  br label %2810

2810:                                             ; preds = %.preheader233, %2848
  %2811 = phi i64 [ %2813, %2848 ], [ 0, %.preheader233 ]
  %2812 = getelementptr inbounds nuw [40 x i8], ptr %2804, i64 %2811
  %2813 = add nuw nsw i64 %2811, 1
  %2814 = load i64, ptr %2812, align 8, !range !1940, !alias.scope !34634, !noalias !34628, !noundef !1708
  %2815 = icmp ugt i64 %2814, 5
  br i1 %2815, label %2816, label %2848

2816:                                             ; preds = %2810
  %2817 = getelementptr i8, ptr %2812, i64 8
  %2818 = load ptr, ptr %2817, align 8, !alias.scope !34631, !noalias !34628, !nonnull !1708, !noundef !1708
  %2819 = shl i64 %2814, 3
  %2820 = add i64 %2819, -8
  %2821 = load i64, ptr %2808, align 8, !noalias !34637, !noundef !1708
  %2822 = call i64 @llvm.umin.i64(i64 %2820, i64 9223372036854775807)
  %2823 = call i64 @llvm.ssub.sat.i64(i64 %2821, i64 %2822)
  store i64 %2823, ptr %2808, align 8, !noalias !34637
  %2824 = load i64, ptr %2809, align 8, !noalias !34637, !noundef !1708
  %2825 = icmp slt i64 %2823, %2824
  br i1 %2825, label %2826, label %.preheader1921

2826:                                             ; preds = %2816
  store i64 %2823, ptr %2809, align 8, !noalias !34637
  br label %.preheader1921

.preheader1921:                                   ; preds = %2826, %2816
  br label %2827

2827:                                             ; preds = %.preheader1921, %2830
  %2828 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952) acquire, align 8, !noalias !34637
  %2829 = icmp slt i64 %2828, 0
  br i1 %2829, label %2830, label %__rustc::__rust_dealloc (.exit223)

2830:                                             ; preds = %2827
  %2831 = add nsw i64 %2828, 1
  %2832 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 %2828, i64 %2831 acq_rel acquire, align 8, !noalias !34637
  %2833 = extractvalue { i64, i1 } %2832, 1
  br i1 %2833, label %2834, label %2827

2834:                                             ; preds = %2830
  %2835 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %2822 monotonic, align 8, !noalias !34637
  %2836 = call i64 @llvm.ssub.sat.i64(i64 %2835, i64 %2822)
  %2837 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !34637
  br label %2838

2838:                                             ; preds = %2841, %2834
  %2839 = phi i64 [ %2837, %2834 ], [ %2844, %2841 ]
  %2840 = icmp slt i64 %2836, %2839
  br i1 %2840, label %2841, label %2845

2841:                                             ; preds = %2838
  %2842 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %2839, i64 %2836 monotonic monotonic, align 8, !noalias !34637
  %2843 = extractvalue { i64, i1 } %2842, 1
  %2844 = extractvalue { i64, i1 } %2842, 0
  br i1 %2843, label %2845, label %2838

2845:                                             ; preds = %2841, %2838
  %2846 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 1 release, align 8, !noalias !34637
  br label %__rustc::__rust_dealloc (.exit223)

__rustc::__rust_dealloc (.exit223): ; preds = %2827, %2845
  %2847 = icmp ne i64 %2820, 0
  call void @llvm.assume(i1 %2847), !noalias !34637
  call void @free(ptr noundef nonnull %2818) #88, !noalias !34637
  br label %2848

2848:                                             ; preds = %__rustc::__rust_dealloc (.exit223), %2810
  %2849 = icmp eq i64 %2813, %2806
  br i1 %2849, label %.loopexit234, label %2810

.loopexit234:                                     ; preds = %2848, %2801
  %2850 = load i64, ptr %121, align 8, !alias.scope !34628
  %2851 = icmp eq i64 %2850, 0
  br i1 %2851, label %2882, label %2852

2852:                                             ; preds = %.loopexit234
  %2853 = mul nuw i64 %2850, 40
  %2854 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %2855 = load i64, ptr %2854, align 8, !noalias !34628, !noundef !1708
  %2856 = call i64 @llvm.umin.i64(i64 %2853, i64 9223372036854775807)
  %2857 = call i64 @llvm.ssub.sat.i64(i64 %2855, i64 %2856)
  store i64 %2857, ptr %2854, align 8, !noalias !34628
  %2858 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %2859 = load i64, ptr %2858, align 8, !noalias !34628, !noundef !1708
  %2860 = icmp slt i64 %2857, %2859
  br i1 %2860, label %2861, label %.preheader1920

2861:                                             ; preds = %2852
  store i64 %2857, ptr %2858, align 8, !noalias !34628
  br label %.preheader1920

.preheader1920:                                   ; preds = %2861, %2852
  br label %2862

2862:                                             ; preds = %.preheader1920, %2865
  %2863 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952) acquire, align 8, !noalias !34628
  %2864 = icmp slt i64 %2863, 0
  br i1 %2864, label %2865, label %__rustc::__rust_dealloc (.exit224)

2865:                                             ; preds = %2862
  %2866 = add nsw i64 %2863, 1
  %2867 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 %2863, i64 %2866 acq_rel acquire, align 8, !noalias !34628
  %2868 = extractvalue { i64, i1 } %2867, 1
  br i1 %2868, label %2869, label %2862

2869:                                             ; preds = %2865
  %2870 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %2856 monotonic, align 8, !noalias !34628
  %2871 = call i64 @llvm.ssub.sat.i64(i64 %2870, i64 %2856)
  %2872 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !34628
  br label %2873

2873:                                             ; preds = %2876, %2869
  %2874 = phi i64 [ %2872, %2869 ], [ %2879, %2876 ]
  %2875 = icmp slt i64 %2871, %2874
  br i1 %2875, label %2876, label %2880

2876:                                             ; preds = %2873
  %2877 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %2874, i64 %2871 monotonic monotonic, align 8, !noalias !34628
  %2878 = extractvalue { i64, i1 } %2877, 1
  %2879 = extractvalue { i64, i1 } %2877, 0
  br i1 %2878, label %2880, label %2873

2880:                                             ; preds = %2876, %2873
  %2881 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 1 release, align 8, !noalias !34628
  br label %__rustc::__rust_dealloc (.exit224)

__rustc::__rust_dealloc (.exit224): ; preds = %2862, %2880
  call void @free(ptr noundef nonnull %2804) #88, !noalias !34628
  br label %2882

2882:                                             ; preds = %2992, %__rustc::__rust_dealloc (.exit224), %.loopexit234, %530
  %2883 = phi i8 [ 1, %2992 ], [ 0, %530 ], [ %534, %.loopexit234 ], [ %534, %__rustc::__rust_dealloc (.exit224) ]
  call void @llvm.lifetime.end.p0(ptr nonnull %121)
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>(ptr noalias nofree noundef align 8 dereferenceable(216) %122)
          to label %2993 unwind label %191

2884:                                             ; preds = %2797, %2794, %2790
  call void @llvm.lifetime.end.p0(ptr nonnull %91)
  call void @llvm.lifetime.end.p0(ptr nonnull %92)
  %2885 = getelementptr inbounds nuw i8, ptr %0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(96) %2885, ptr noundef nonnull align 8 dereferenceable(96) %93, i64 96, i1 false)
  store i64 0, ptr %0, align 16
  call void @llvm.lifetime.end.p0(ptr nonnull %93)
  call void @llvm.lifetime.end.p0(ptr nonnull %121)
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>(ptr noalias nofree noundef align 8 dereferenceable(216) %122)
          to label %2886 unwind label %191

2886:                                             ; preds = %2884
  call void @llvm.lifetime.end.p0(ptr nonnull %122)
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(200) %123)
          to label %2887 unwind label %157

2887:                                             ; preds = %2886
  call void @llvm.lifetime.end.p0(ptr nonnull %123)
  call void @llvm.lifetime.end.p0(ptr nonnull %124)
  call void @llvm.experimental.noalias.scope.decl(metadata !34640)
  call void @llvm.experimental.noalias.scope.decl(metadata !34643)
  %2888 = load ptr, ptr %144, align 8, !alias.scope !34646, !nonnull !1708, !noundef !1708
  %2889 = atomicrmw sub ptr %2888, i64 1 release, align 8, !noalias !34646
  %2890 = icmp eq i64 %2889, 1
  br i1 %2890, label %2891, label %2895

2891:                                             ; preds = %2887
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %144) #87
          to label %2895 unwind label %2892

2892:                                             ; preds = %2891
  %2893 = landingpad { ptr, i32 }
          cleanup
  %2894 = trunc nuw i8 %534 to i1
  br i1 %2894, label %2978, label %3191

2895:                                             ; preds = %2891, %2887
  %2896 = trunc nuw i8 %534 to i1
  br i1 %2896, label %2898, label %2897

2897:                                             ; preds = %__rustc::__rust_dealloc (.exit226), %.loopexit232, %2895
  call void @llvm.lifetime.end.p0(ptr nonnull %127)
  br label %2977

2898:                                             ; preds = %2895
  call void @llvm.experimental.noalias.scope.decl(metadata !34647)
  %2899 = getelementptr inbounds nuw i8, ptr %127, i64 8
  %2900 = load ptr, ptr %2899, align 8, !alias.scope !34647, !nonnull !1708, !noundef !1708
  %2901 = load i64, ptr %182, align 8, !alias.scope !34647, !noundef !1708
  call void @llvm.experimental.noalias.scope.decl(metadata !34650)
  %2902 = icmp eq i64 %2901, 0
  br i1 %2902, label %.loopexit232, label %.preheader231

.preheader231:                                    ; preds = %2898
  %2903 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %2904 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  br label %2905

2905:                                             ; preds = %.preheader231, %2943
  %2906 = phi i64 [ %2908, %2943 ], [ 0, %.preheader231 ]
  %2907 = getelementptr inbounds nuw [40 x i8], ptr %2900, i64 %2906
  %2908 = add nuw nsw i64 %2906, 1
  %2909 = load i64, ptr %2907, align 8, !range !1940, !alias.scope !34653, !noalias !34647, !noundef !1708
  %2910 = icmp ugt i64 %2909, 5
  br i1 %2910, label %2911, label %2943

2911:                                             ; preds = %2905
  %2912 = getelementptr i8, ptr %2907, i64 8
  %2913 = load ptr, ptr %2912, align 8, !alias.scope !34650, !noalias !34647, !nonnull !1708, !noundef !1708
  %2914 = shl i64 %2909, 3
  %2915 = add i64 %2914, -8
  %2916 = load i64, ptr %2903, align 8, !noalias !34656, !noundef !1708
  %2917 = call i64 @llvm.umin.i64(i64 %2915, i64 9223372036854775807)
  %2918 = call i64 @llvm.ssub.sat.i64(i64 %2916, i64 %2917)
  store i64 %2918, ptr %2903, align 8, !noalias !34656
  %2919 = load i64, ptr %2904, align 8, !noalias !34656, !noundef !1708
  %2920 = icmp slt i64 %2918, %2919
  br i1 %2920, label %2921, label %.preheader1918

2921:                                             ; preds = %2911
  store i64 %2918, ptr %2904, align 8, !noalias !34656
  br label %.preheader1918

.preheader1918:                                   ; preds = %2921, %2911
  br label %2922

2922:                                             ; preds = %.preheader1918, %2925
  %2923 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952) acquire, align 8, !noalias !34656
  %2924 = icmp slt i64 %2923, 0
  br i1 %2924, label %2925, label %__rustc::__rust_dealloc (.exit225)

2925:                                             ; preds = %2922
  %2926 = add nsw i64 %2923, 1
  %2927 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 %2923, i64 %2926 acq_rel acquire, align 8, !noalias !34656
  %2928 = extractvalue { i64, i1 } %2927, 1
  br i1 %2928, label %2929, label %2922

2929:                                             ; preds = %2925
  %2930 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %2917 monotonic, align 8, !noalias !34656
  %2931 = call i64 @llvm.ssub.sat.i64(i64 %2930, i64 %2917)
  %2932 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !34656
  br label %2933

2933:                                             ; preds = %2936, %2929
  %2934 = phi i64 [ %2932, %2929 ], [ %2939, %2936 ]
  %2935 = icmp slt i64 %2931, %2934
  br i1 %2935, label %2936, label %2940

2936:                                             ; preds = %2933
  %2937 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %2934, i64 %2931 monotonic monotonic, align 8, !noalias !34656
  %2938 = extractvalue { i64, i1 } %2937, 1
  %2939 = extractvalue { i64, i1 } %2937, 0
  br i1 %2938, label %2940, label %2933

2940:                                             ; preds = %2936, %2933
  %2941 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 1 release, align 8, !noalias !34656
  br label %__rustc::__rust_dealloc (.exit225)

__rustc::__rust_dealloc (.exit225): ; preds = %2922, %2940
  %2942 = icmp ne i64 %2915, 0
  call void @llvm.assume(i1 %2942), !noalias !34656
  call void @free(ptr noundef nonnull %2913) #88, !noalias !34656
  br label %2943

2943:                                             ; preds = %__rustc::__rust_dealloc (.exit225), %2905
  %2944 = icmp eq i64 %2908, %2901
  br i1 %2944, label %.loopexit232, label %2905

.loopexit232:                                     ; preds = %2943, %2898
  %2945 = load i64, ptr %127, align 8, !alias.scope !34647
  %2946 = icmp eq i64 %2945, 0
  br i1 %2946, label %2897, label %2947

2947:                                             ; preds = %.loopexit232
  %2948 = mul nuw i64 %2945, 40
  %2949 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %2950 = load i64, ptr %2949, align 8, !noalias !34647, !noundef !1708
  %2951 = call i64 @llvm.umin.i64(i64 %2948, i64 9223372036854775807)
  %2952 = call i64 @llvm.ssub.sat.i64(i64 %2950, i64 %2951)
  store i64 %2952, ptr %2949, align 8, !noalias !34647
  %2953 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %2954 = load i64, ptr %2953, align 8, !noalias !34647, !noundef !1708
  %2955 = icmp slt i64 %2952, %2954
  br i1 %2955, label %2956, label %.preheader1917

2956:                                             ; preds = %2947
  store i64 %2952, ptr %2953, align 8, !noalias !34647
  br label %.preheader1917

.preheader1917:                                   ; preds = %2956, %2947
  br label %2957

2957:                                             ; preds = %.preheader1917, %2960
  %2958 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952) acquire, align 8, !noalias !34647
  %2959 = icmp slt i64 %2958, 0
  br i1 %2959, label %2960, label %__rustc::__rust_dealloc (.exit226)

2960:                                             ; preds = %2957
  %2961 = add nsw i64 %2958, 1
  %2962 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 %2958, i64 %2961 acq_rel acquire, align 8, !noalias !34647
  %2963 = extractvalue { i64, i1 } %2962, 1
  br i1 %2963, label %2964, label %2957

2964:                                             ; preds = %2960
  %2965 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %2951 monotonic, align 8, !noalias !34647
  %2966 = call i64 @llvm.ssub.sat.i64(i64 %2965, i64 %2951)
  %2967 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !34647
  br label %2968

2968:                                             ; preds = %2971, %2964
  %2969 = phi i64 [ %2967, %2964 ], [ %2974, %2971 ]
  %2970 = icmp slt i64 %2966, %2969
  br i1 %2970, label %2971, label %2975

2971:                                             ; preds = %2968
  %2972 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %2969, i64 %2966 monotonic monotonic, align 8, !noalias !34647
  %2973 = extractvalue { i64, i1 } %2972, 1
  %2974 = extractvalue { i64, i1 } %2972, 0
  br i1 %2973, label %2975, label %2968

2975:                                             ; preds = %2971, %2968
  %2976 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 1 release, align 8, !noalias !34647
  br label %__rustc::__rust_dealloc (.exit226)

__rustc::__rust_dealloc (.exit226): ; preds = %2957, %2975
  call void @free(ptr noundef nonnull %2900) #88, !noalias !34647
  br label %2897

2977:                                             ; preds = %3179, %3176, %3172, %2897, %148
  ret void

2978:                                             ; preds = %2892
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %127) #89
  br label %3191

2979:                                             ; preds = %2798
  %2980 = landingpad { ptr, i32 }
          cleanup
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %121) #89
  br label %524

2981:                                             ; preds = %2728, %960
  call void @llvm.experimental.noalias.scope.decl(metadata !34659)
  %2982 = load ptr, ptr %120, align 8, !alias.scope !34659, !noundef !1708
  %2983 = icmp eq ptr %2982, null
  br i1 %2983, label %2992, label %2984

2984:                                             ; preds = %2981
  %2985 = atomicrmw sub ptr %2982, i64 1 release, align 8, !noalias !34662
  %2986 = icmp eq i64 %2985, 1
  br i1 %2986, label %2987, label %2992

2987:                                             ; preds = %2984
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %120) #87
          to label %2992 unwind label %528, !inline_history !1744

2988:                                             ; preds = %2382, %2375, %2331, %2293, %._crit_edge1769, %2270, %2269, %1337, %999
  %2989 = phi { ptr, i32 } [ %1000, %999 ], [ %2286, %._crit_edge1769 ], [ %2286, %2293 ], [ %2332, %2331 ], [ %1452, %2270 ], [ %1338, %1337 ], [ %1452, %2269 ], [ %2383, %2382 ], [ %2376, %2375 ]
; invoke core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>>(ptr noalias nofree noundef align 8 dereferenceable(32) %114) #89
          to label %566 unwind label %545

2990:                                             ; preds = %966
  %2991 = landingpad { ptr, i32 }
          cleanup
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %119) #89
  br label %566

2992:                                             ; preds = %2987, %2984, %2981
  call void @llvm.lifetime.end.p0(ptr nonnull %120)
  br label %2882

2993:                                             ; preds = %2882
  call void @llvm.lifetime.end.p0(ptr nonnull %122)
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(200) %123)
          to label %2994 unwind label %154

2994:                                             ; preds = %2993
  call void @llvm.lifetime.end.p0(ptr nonnull %123)
  call void @llvm.experimental.noalias.scope.decl(metadata !34665)
  call void @llvm.experimental.noalias.scope.decl(metadata !34668)
  %2995 = load ptr, ptr %124, align 8, !alias.scope !34671, !nonnull !1708, !noundef !1708
  %2996 = atomicrmw sub ptr %2995, i64 1 release, align 8, !noalias !34671
  %2997 = icmp eq i64 %2996, 1
  br i1 %2997, label %2998, label %3009

2998:                                             ; preds = %2994
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %124) #87
          to label %3009 unwind label %3007

2999:                                             ; preds = %3187, %3181, %3007, %157, %153
  %3000 = phi i8 [ %2883, %3007 ], [ %188, %153 ], [ %3183, %3187 ], [ %3183, %3181 ], [ %534, %157 ]
  %3001 = phi i1 [ true, %3007 ], [ false, %153 ], [ true, %3187 ], [ true, %3181 ], [ false, %157 ]
  %3002 = phi { ptr, i32 } [ %3008, %3007 ], [ %190, %153 ], [ %3182, %3187 ], [ %3182, %3181 ], [ %158, %157 ]
  call void @llvm.experimental.noalias.scope.decl(metadata !34672)
  call void @llvm.experimental.noalias.scope.decl(metadata !34675)
  %3003 = load ptr, ptr %144, align 8, !alias.scope !34678, !nonnull !1708, !noundef !1708
  %3004 = atomicrmw sub ptr %3003, i64 1 release, align 8, !noalias !34678
  %3005 = icmp eq i64 %3004, 1
  br i1 %3005, label %3006, label %3188

3006:                                             ; preds = %2999
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %144) #87
          to label %3188 unwind label %545

3007:                                             ; preds = %2998
  %3008 = landingpad { ptr, i32 }
          cleanup
  br label %2999

3009:                                             ; preds = %2998, %2994
  call void @llvm.lifetime.end.p0(ptr nonnull %124)
  call void @llvm.experimental.noalias.scope.decl(metadata !34679)
  call void @llvm.experimental.noalias.scope.decl(metadata !34682)
  %3010 = load ptr, ptr %144, align 8, !alias.scope !34685, !nonnull !1708, !noundef !1708
  %3011 = atomicrmw sub ptr %3010, i64 1 release, align 8, !noalias !34685
  %3012 = icmp eq i64 %3011, 1
  br i1 %3012, label %3013, label %3017

3013:                                             ; preds = %3009
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %144) #87
          to label %3017 unwind label %3014

3014:                                             ; preds = %3013
  %3015 = landingpad { ptr, i32 }
          cleanup
  %3016 = trunc nuw i8 %2883 to i1
  br i1 %3016, label %3180, label %3193

3017:                                             ; preds = %3013, %3009
  %3018 = trunc nuw i8 %2883 to i1
  br i1 %3018, label %3020, label %3019

3019:                                             ; preds = %__rustc::__rust_dealloc (.exit228), %.loopexit, %3017
  call void @llvm.lifetime.end.p0(ptr nonnull %127)
  br label %3099

3020:                                             ; preds = %3017
  call void @llvm.experimental.noalias.scope.decl(metadata !34686)
  %3021 = getelementptr inbounds nuw i8, ptr %127, i64 8
  %3022 = load ptr, ptr %3021, align 8, !alias.scope !34686, !nonnull !1708, !noundef !1708
  %3023 = load i64, ptr %182, align 8, !alias.scope !34686, !noundef !1708
  call void @llvm.experimental.noalias.scope.decl(metadata !34689)
  %3024 = icmp eq i64 %3023, 0
  br i1 %3024, label %.loopexit, label %.preheader

.preheader:                                       ; preds = %3020
  %3025 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %3026 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  br label %3027

3027:                                             ; preds = %.preheader, %3065
  %3028 = phi i64 [ %3030, %3065 ], [ 0, %.preheader ]
  %3029 = getelementptr inbounds nuw [40 x i8], ptr %3022, i64 %3028
  %3030 = add nuw nsw i64 %3028, 1
  %3031 = load i64, ptr %3029, align 8, !range !1940, !alias.scope !34692, !noalias !34686, !noundef !1708
  %3032 = icmp ugt i64 %3031, 5
  br i1 %3032, label %3033, label %3065

3033:                                             ; preds = %3027
  %3034 = getelementptr i8, ptr %3029, i64 8
  %3035 = load ptr, ptr %3034, align 8, !alias.scope !34689, !noalias !34686, !nonnull !1708, !noundef !1708
  %3036 = shl i64 %3031, 3
  %3037 = add i64 %3036, -8
  %3038 = load i64, ptr %3025, align 8, !noalias !34695, !noundef !1708
  %3039 = call i64 @llvm.umin.i64(i64 %3037, i64 9223372036854775807)
  %3040 = call i64 @llvm.ssub.sat.i64(i64 %3038, i64 %3039)
  store i64 %3040, ptr %3025, align 8, !noalias !34695
  %3041 = load i64, ptr %3026, align 8, !noalias !34695, !noundef !1708
  %3042 = icmp slt i64 %3040, %3041
  br i1 %3042, label %3043, label %.preheader1916

3043:                                             ; preds = %3033
  store i64 %3040, ptr %3026, align 8, !noalias !34695
  br label %.preheader1916

.preheader1916:                                   ; preds = %3043, %3033
  br label %3044

3044:                                             ; preds = %.preheader1916, %3047
  %3045 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952) acquire, align 8, !noalias !34695
  %3046 = icmp slt i64 %3045, 0
  br i1 %3046, label %3047, label %__rustc::__rust_dealloc (.exit227)

3047:                                             ; preds = %3044
  %3048 = add nsw i64 %3045, 1
  %3049 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 %3045, i64 %3048 acq_rel acquire, align 8, !noalias !34695
  %3050 = extractvalue { i64, i1 } %3049, 1
  br i1 %3050, label %3051, label %3044

3051:                                             ; preds = %3047
  %3052 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %3039 monotonic, align 8, !noalias !34695
  %3053 = call i64 @llvm.ssub.sat.i64(i64 %3052, i64 %3039)
  %3054 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !34695
  br label %3055

3055:                                             ; preds = %3058, %3051
  %3056 = phi i64 [ %3054, %3051 ], [ %3061, %3058 ]
  %3057 = icmp slt i64 %3053, %3056
  br i1 %3057, label %3058, label %3062

3058:                                             ; preds = %3055
  %3059 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %3056, i64 %3053 monotonic monotonic, align 8, !noalias !34695
  %3060 = extractvalue { i64, i1 } %3059, 1
  %3061 = extractvalue { i64, i1 } %3059, 0
  br i1 %3060, label %3062, label %3055

3062:                                             ; preds = %3058, %3055
  %3063 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 1 release, align 8, !noalias !34695
  br label %__rustc::__rust_dealloc (.exit227)

__rustc::__rust_dealloc (.exit227): ; preds = %3044, %3062
  %3064 = icmp ne i64 %3037, 0
  call void @llvm.assume(i1 %3064), !noalias !34695
  call void @free(ptr noundef nonnull %3035) #88, !noalias !34695
  br label %3065

3065:                                             ; preds = %__rustc::__rust_dealloc (.exit227), %3027
  %3066 = icmp eq i64 %3030, %3023
  br i1 %3066, label %.loopexit, label %3027

.loopexit:                                        ; preds = %3065, %3020
  %3067 = load i64, ptr %127, align 8, !alias.scope !34686
  %3068 = icmp eq i64 %3067, 0
  br i1 %3068, label %3019, label %3069

3069:                                             ; preds = %.loopexit
  %3070 = mul nuw i64 %3067, 40
  %3071 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %3072 = load i64, ptr %3071, align 8, !noalias !34686, !noundef !1708
  %3073 = call i64 @llvm.umin.i64(i64 %3070, i64 9223372036854775807)
  %3074 = call i64 @llvm.ssub.sat.i64(i64 %3072, i64 %3073)
  store i64 %3074, ptr %3071, align 8, !noalias !34686
  %3075 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %3076 = load i64, ptr %3075, align 8, !noalias !34686, !noundef !1708
  %3077 = icmp slt i64 %3074, %3076
  br i1 %3077, label %3078, label %.preheader1915

3078:                                             ; preds = %3069
  store i64 %3074, ptr %3075, align 8, !noalias !34686
  br label %.preheader1915

.preheader1915:                                   ; preds = %3078, %3069
  br label %3079

3079:                                             ; preds = %.preheader1915, %3082
  %3080 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952) acquire, align 8, !noalias !34686
  %3081 = icmp slt i64 %3080, 0
  br i1 %3081, label %3082, label %__rustc::__rust_dealloc (.exit228)

3082:                                             ; preds = %3079
  %3083 = add nsw i64 %3080, 1
  %3084 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 %3080, i64 %3083 acq_rel acquire, align 8, !noalias !34686
  %3085 = extractvalue { i64, i1 } %3084, 1
  br i1 %3085, label %3086, label %3079

3086:                                             ; preds = %3082
  %3087 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %3073 monotonic, align 8, !noalias !34686
  %3088 = call i64 @llvm.ssub.sat.i64(i64 %3087, i64 %3073)
  %3089 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !34686
  br label %3090

3090:                                             ; preds = %3093, %3086
  %3091 = phi i64 [ %3089, %3086 ], [ %3096, %3093 ]
  %3092 = icmp slt i64 %3088, %3091
  br i1 %3092, label %3093, label %3097

3093:                                             ; preds = %3090
  %3094 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %3091, i64 %3088 monotonic monotonic, align 8, !noalias !34686
  %3095 = extractvalue { i64, i1 } %3094, 1
  %3096 = extractvalue { i64, i1 } %3094, 0
  br i1 %3095, label %3097, label %3090

3097:                                             ; preds = %3093, %3090
  %3098 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 1 release, align 8, !noalias !34686
  br label %__rustc::__rust_dealloc (.exit228)

__rustc::__rust_dealloc (.exit228): ; preds = %3079, %3097
  call void @free(ptr noundef nonnull %3022) #88, !noalias !34686
  br label %3019

3099:                                             ; preds = %3019, %135
  %3100 = getelementptr inbounds nuw i8, ptr %128, i64 72
  %3101 = load i64, ptr %3100, align 8, !range !1940, !noundef !1708
  %3102 = icmp ugt i64 %3101, 5
  br i1 %3102, label %3103, label %3137

3103:                                             ; preds = %3099
  %3104 = getelementptr inbounds nuw i8, ptr %128, i64 80
  %3105 = load ptr, ptr %3104, align 8, !nonnull !1708, !noundef !1708
  %3106 = mul i64 %3101, 3
  %3107 = add i64 %3106, -3
  %3108 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %3109 = load i64, ptr %3108, align 8, !noalias !34698, !noundef !1708
  %3110 = call i64 @llvm.umin.i64(i64 %3107, i64 9223372036854775807)
  %3111 = call i64 @llvm.ssub.sat.i64(i64 %3109, i64 %3110)
  store i64 %3111, ptr %3108, align 8, !noalias !34698
  %3112 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %3113 = load i64, ptr %3112, align 8, !noalias !34698, !noundef !1708
  %3114 = icmp slt i64 %3111, %3113
  br i1 %3114, label %3115, label %.preheader1914

3115:                                             ; preds = %3103
  store i64 %3111, ptr %3112, align 8, !noalias !34698
  br label %.preheader1914

.preheader1914:                                   ; preds = %3115, %3103
  br label %3116

3116:                                             ; preds = %.preheader1914, %3119
  %3117 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952) acquire, align 8, !noalias !34698
  %3118 = icmp slt i64 %3117, 0
  br i1 %3118, label %3119, label %__rustc::__rust_dealloc (.exit229)

3119:                                             ; preds = %3116
  %3120 = add nsw i64 %3117, 1
  %3121 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 %3117, i64 %3120 acq_rel acquire, align 8, !noalias !34698
  %3122 = extractvalue { i64, i1 } %3121, 1
  br i1 %3122, label %3123, label %3116

3123:                                             ; preds = %3119
  %3124 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %3110 monotonic, align 8, !noalias !34698
  %3125 = call i64 @llvm.ssub.sat.i64(i64 %3124, i64 %3110)
  %3126 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !34698
  br label %3127

3127:                                             ; preds = %3130, %3123
  %3128 = phi i64 [ %3126, %3123 ], [ %3133, %3130 ]
  %3129 = icmp slt i64 %3125, %3128
  br i1 %3129, label %3130, label %3134

3130:                                             ; preds = %3127
  %3131 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %3128, i64 %3125 monotonic monotonic, align 8, !noalias !34698
  %3132 = extractvalue { i64, i1 } %3131, 1
  %3133 = extractvalue { i64, i1 } %3131, 0
  br i1 %3132, label %3134, label %3127

3134:                                             ; preds = %3130, %3127
  %3135 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 1 release, align 8, !noalias !34698
  br label %__rustc::__rust_dealloc (.exit229)

__rustc::__rust_dealloc (.exit229): ; preds = %3116, %3134
  %3136 = icmp ne i64 %3107, 0
  call void @llvm.assume(i1 %3136), !noalias !34698
  call void @free(ptr noundef nonnull %3105) #88, !noalias !34698
  br label %3137

3137:                                             ; preds = %__rustc::__rust_dealloc (.exit229), %3099
  %3138 = load i64, ptr %128, align 8, !range !2062, !noundef !1708
  %3139 = icmp sgt i64 %3138, 0
  br i1 %3139, label %3140, label %3172

3140:                                             ; preds = %3137
  %3141 = getelementptr inbounds nuw i8, ptr %128, i64 8
  %3142 = load ptr, ptr %3141, align 8, !nonnull !1708, !noundef !1708
  %3143 = mul nuw i64 %3138, 3
  %3144 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %3145 = load i64, ptr %3144, align 8, !noalias !34703, !noundef !1708
  %3146 = call i64 @llvm.umin.i64(i64 %3143, i64 9223372036854775807)
  %3147 = call i64 @llvm.ssub.sat.i64(i64 %3145, i64 %3146)
  store i64 %3147, ptr %3144, align 8, !noalias !34703
  %3148 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %3149 = load i64, ptr %3148, align 8, !noalias !34703, !noundef !1708
  %3150 = icmp slt i64 %3147, %3149
  br i1 %3150, label %3151, label %.preheader1913

3151:                                             ; preds = %3140
  store i64 %3147, ptr %3148, align 8, !noalias !34703
  br label %.preheader1913

.preheader1913:                                   ; preds = %3151, %3140
  br label %3152

3152:                                             ; preds = %.preheader1913, %3155
  %3153 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952) acquire, align 8, !noalias !34703
  %3154 = icmp slt i64 %3153, 0
  br i1 %3154, label %3155, label %__rustc::__rust_dealloc (.exit230)

3155:                                             ; preds = %3152
  %3156 = add nsw i64 %3153, 1
  %3157 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 %3153, i64 %3156 acq_rel acquire, align 8, !noalias !34703
  %3158 = extractvalue { i64, i1 } %3157, 1
  br i1 %3158, label %3159, label %3152

3159:                                             ; preds = %3155
  %3160 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %3146 monotonic, align 8, !noalias !34703
  %3161 = call i64 @llvm.ssub.sat.i64(i64 %3160, i64 %3146)
  %3162 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !34703
  br label %3163

3163:                                             ; preds = %3166, %3159
  %3164 = phi i64 [ %3162, %3159 ], [ %3169, %3166 ]
  %3165 = icmp slt i64 %3161, %3164
  br i1 %3165, label %3166, label %3170

3166:                                             ; preds = %3163
  %3167 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %3164, i64 %3161 monotonic monotonic, align 8, !noalias !34703
  %3168 = extractvalue { i64, i1 } %3167, 1
  %3169 = extractvalue { i64, i1 } %3167, 0
  br i1 %3168, label %3170, label %3163

3170:                                             ; preds = %3166, %3163
  %3171 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 1 release, align 8, !noalias !34703
  br label %__rustc::__rust_dealloc (.exit230)

__rustc::__rust_dealloc (.exit230): ; preds = %3152, %3170
  call void @free(ptr noundef nonnull %3142) #88, !noalias !34703
  br label %3172

3172:                                             ; preds = %__rustc::__rust_dealloc (.exit230), %3137
  %3173 = getelementptr inbounds nuw i8, ptr %128, i64 96
  %3174 = load ptr, ptr %3173, align 8, !noundef !1708
  %3175 = icmp eq ptr %3174, null
  br i1 %3175, label %2977, label %3176

3176:                                             ; preds = %3172
  %3177 = atomicrmw sub ptr %3174, i64 1 release, align 8, !noalias !34704
  %3178 = icmp eq i64 %3177, 1
  br i1 %3178, label %3179, label %2977

3179:                                             ; preds = %3176
  fence acquire
; call <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %3173) #87
  br label %2977

3180:                                             ; preds = %3014
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %127) #89
  br label %3193

3181:                                             ; preds = %154, %153
  %3182 = phi { ptr, i32 } [ %156, %154 ], [ %190, %153 ]
  %3183 = phi i8 [ %155, %154 ], [ %188, %153 ]
  call void @llvm.experimental.noalias.scope.decl(metadata !34711)
  call void @llvm.experimental.noalias.scope.decl(metadata !34714)
  %3184 = load ptr, ptr %124, align 8, !alias.scope !34717, !nonnull !1708, !noundef !1708
  %3185 = atomicrmw sub ptr %3184, i64 1 release, align 8, !noalias !34717
  %3186 = icmp eq i64 %3185, 1
  br i1 %3186, label %3187, label %2999

3187:                                             ; preds = %3181
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %124) #87
          to label %2999 unwind label %545

3188:                                             ; preds = %3006, %2999
  %3189 = trunc nuw i8 %3000 to i1
  br i1 %3189, label %3190, label %129

3190:                                             ; preds = %3188
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %127) #89
  br i1 %3001, label %3193, label %3191

3191:                                             ; preds = %3193, %3190, %2978, %2892, %129
  %3192 = phi { ptr, i32 } [ %3194, %3193 ], [ %3002, %129 ], [ %2893, %2978 ], [ %2893, %2892 ], [ %3002, %3190 ]
  resume { ptr, i32 } %3192

3193:                                             ; preds = %3190, %3180, %3014, %130, %129
  %3194 = phi { ptr, i32 } [ %3002, %129 ], [ %3015, %3180 ], [ %3015, %3014 ], [ %131, %130 ], [ %3002, %3190 ]
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>(ptr noalias nofree noundef align 8 dereferenceable(104) %128) #89
          to label %3191 unwind label %545
}

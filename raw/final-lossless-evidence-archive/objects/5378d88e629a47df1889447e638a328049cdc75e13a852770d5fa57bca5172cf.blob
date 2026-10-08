define void @purrdf_sparql_eval::expr::eval_filter::<purrdf_core::ir::dataset::RdfDataset>(ptr dead_on_unwind noalias nofree noundef writable writeonly sret([112 x i8]) align 16 captures(none) dereferenceable(112) %0, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(144) %1, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(64) %2, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(144) %3, ptr noalias nofree noundef align 16 dereferenceable(1232) %4) unnamed_addr #8 personality ptr @rust_eh_personality !guid !23453 {
  %6 = alloca [96 x i8], align 16
  %7 = alloca [96 x i8], align 16
  %8 = alloca [96 x i8], align 16
  %9 = alloca [96 x i8], align 16
  %10 = alloca [40 x i8], align 8
  %11 = alloca [24 x i8], align 8
  %12 = alloca [48 x i8], align 8
  %13 = alloca [32 x i8], align 8
  %14 = alloca [104 x i8], align 8
  %15 = alloca [96 x i8], align 8
  %16 = alloca [96 x i8], align 8
  %17 = alloca [24 x i8], align 8
  %18 = alloca [24 x i8], align 8
  %19 = alloca [80 x i8], align 16
  %20 = alloca [24 x i8], align 8
  %21 = alloca [40 x i8], align 8
  %22 = alloca [32 x i8], align 8
  %23 = alloca [32 x i8], align 8
  %24 = alloca [24 x i8], align 8
  %25 = alloca [16 x i8], align 8
  %26 = alloca [80 x i8], align 16
  %27 = alloca [24 x i8], align 8
  %28 = alloca [200 x i8], align 8
  %29 = alloca [24 x i8], align 8
  %30 = alloca [96 x i8], align 16
  %31 = alloca [24 x i8], align 8
  %32 = alloca [24 x i8], align 8
  %33 = alloca [248 x i8], align 8
  %34 = alloca [240 x i8], align 8
  %35 = alloca [208 x i8], align 8
  %36 = alloca [32 x i8], align 8
  %37 = alloca [24 x i8], align 8
  %38 = alloca [32 x i8], align 8
  %39 = alloca [256 x i8], align 16
  %40 = alloca [208 x i8], align 16
  %41 = alloca [8 x i8], align 8
  %42 = alloca [24 x i8], align 8
  %43 = alloca [216 x i8], align 8
  %44 = alloca [200 x i8], align 8
  %45 = alloca [8 x i8], align 8
  %46 = alloca [112 x i8], align 16
  %47 = alloca [104 x i8], align 8
  %48 = alloca [104 x i8], align 8
  %49 = alloca [32 x i8], align 8
  %50 = alloca [32 x i8], align 8
  %51 = alloca [104 x i8], align 8
; call <purrdf_sparql_eval::governor::lift::Lift>::at
  call void @<purrdf_sparql_eval::governor::lift::Lift>::at(ptr noalias nofree noundef nonnull sret([104 x i8]) align 8 captures(none) dereferenceable(104) %51, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %1)
  call void @llvm.lifetime.start.p0(ptr nonnull %49)
  call void @llvm.lifetime.start.p0(ptr nonnull %48)
; invoke purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
  invoke fastcc void @purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>(ptr noalias nofree noundef nonnull align 16 captures(none) dereferenceable(112) %46, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %3, ptr noalias nofree noundef nonnull align 16 dereferenceable(1232) %4, ptr noundef nonnull readonly align 8 dereferenceable(144) %3)
          to label %52 unwind label %1368, !inline_history !14077

52:                                               ; preds = %5
  %53 = load i64, ptr %46, align 16, !range !1732, !noundef !1733
  %54 = trunc nuw i64 %53 to i1
  br i1 %54, label %55, label %138

55:                                               ; preds = %52
  %56 = getelementptr inbounds nuw i8, ptr %46, i64 16
  %57 = getelementptr inbounds nuw i8, ptr %0, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(96) %57, ptr noundef nonnull align 16 dereferenceable(96) %56, i64 96, i1 false)
  store i64 1, ptr %0, align 16
  call void @llvm.lifetime.end.p0(ptr nonnull %48)
  call void @llvm.lifetime.end.p0(ptr nonnull %49)
  %58 = getelementptr inbounds nuw i8, ptr %51, i64 72
  %59 = load i64, ptr %58, align 8, !range !1771, !noundef !1733
  %60 = icmp ugt i64 %59, 5
  br i1 %60, label %61, label %95

61:                                               ; preds = %55
  %62 = getelementptr inbounds nuw i8, ptr %51, i64 80
  %63 = load ptr, ptr %62, align 8, !nonnull !1733, !noundef !1733
  %64 = mul i64 %59, 3
  %65 = add i64 %64, -3
  %66 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %67 = load i64, ptr %66, align 8, !noalias !23454, !noundef !1733
  %68 = tail call i64 @llvm.umin.i64(i64 %65, i64 9223372036854775807)
  %69 = tail call i64 @llvm.ssub.sat.i64(i64 %67, i64 %68)
  store i64 %69, ptr %66, align 8, !noalias !23454
  %70 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %71 = load i64, ptr %70, align 8, !noalias !23454, !noundef !1733
  %72 = icmp slt i64 %69, %71
  br i1 %72, label %73, label %.preheader399

73:                                               ; preds = %61
  store i64 %69, ptr %70, align 8, !noalias !23454
  br label %.preheader399

.preheader399:                                    ; preds = %73, %61
  br label %74

74:                                               ; preds = %.preheader399, %77
  %75 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !23454
  %76 = icmp slt i64 %75, 0
  br i1 %76, label %77, label %__rustc::__rust_dealloc (.exit)

77:                                               ; preds = %74
  %78 = add nsw i64 %75, 1
  %79 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %75, i64 %78 acq_rel acquire, align 8, !noalias !23454
  %80 = extractvalue { i64, i1 } %79, 1
  br i1 %80, label %81, label %74

81:                                               ; preds = %77
  %82 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %68 monotonic, align 8, !noalias !23454
  %83 = tail call i64 @llvm.ssub.sat.i64(i64 %82, i64 %68)
  %84 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !23454
  br label %85

85:                                               ; preds = %88, %81
  %86 = phi i64 [ %84, %81 ], [ %91, %88 ]
  %87 = icmp slt i64 %83, %86
  br i1 %87, label %88, label %92

88:                                               ; preds = %85
  %89 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %86, i64 %83 monotonic monotonic, align 8, !noalias !23454
  %90 = extractvalue { i64, i1 } %89, 1
  %91 = extractvalue { i64, i1 } %89, 0
  br i1 %90, label %92, label %85

92:                                               ; preds = %88, %85
  %93 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !23454
  br label %__rustc::__rust_dealloc (.exit)

__rustc::__rust_dealloc (.exit): ; preds = %74, %92
  %94 = icmp ne i64 %65, 0
  tail call void @llvm.assume(i1 %94), !noalias !23454
  tail call void @free(ptr noundef nonnull %63) #92, !noalias !23454
  br label %95

95:                                               ; preds = %__rustc::__rust_dealloc (.exit), %55
  %96 = load i64, ptr %51, align 8, !range !2052, !noundef !1733
  %97 = icmp sgt i64 %96, 0
  br i1 %97, label %98, label %130

98:                                               ; preds = %95
  %99 = getelementptr inbounds nuw i8, ptr %51, i64 8
  %100 = load ptr, ptr %99, align 8, !nonnull !1733, !noundef !1733
  %101 = mul nuw i64 %96, 3
  %102 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %103 = load i64, ptr %102, align 8, !noalias !23459, !noundef !1733
  %104 = tail call i64 @llvm.umin.i64(i64 %101, i64 9223372036854775807)
  %105 = tail call i64 @llvm.ssub.sat.i64(i64 %103, i64 %104)
  store i64 %105, ptr %102, align 8, !noalias !23459
  %106 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %107 = load i64, ptr %106, align 8, !noalias !23459, !noundef !1733
  %108 = icmp slt i64 %105, %107
  br i1 %108, label %109, label %.preheader398

109:                                              ; preds = %98
  store i64 %105, ptr %106, align 8, !noalias !23459
  br label %.preheader398

.preheader398:                                    ; preds = %109, %98
  br label %110

110:                                              ; preds = %.preheader398, %113
  %111 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !23459
  %112 = icmp slt i64 %111, 0
  br i1 %112, label %113, label %__rustc::__rust_dealloc (.exit54)

113:                                              ; preds = %110
  %114 = add nsw i64 %111, 1
  %115 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %111, i64 %114 acq_rel acquire, align 8, !noalias !23459
  %116 = extractvalue { i64, i1 } %115, 1
  br i1 %116, label %117, label %110

117:                                              ; preds = %113
  %118 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %104 monotonic, align 8, !noalias !23459
  %119 = tail call i64 @llvm.ssub.sat.i64(i64 %118, i64 %104)
  %120 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !23459
  br label %121

121:                                              ; preds = %124, %117
  %122 = phi i64 [ %120, %117 ], [ %127, %124 ]
  %123 = icmp slt i64 %119, %122
  br i1 %123, label %124, label %128

124:                                              ; preds = %121
  %125 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %122, i64 %119 monotonic monotonic, align 8, !noalias !23459
  %126 = extractvalue { i64, i1 } %125, 1
  %127 = extractvalue { i64, i1 } %125, 0
  br i1 %126, label %128, label %121

128:                                              ; preds = %124, %121
  %129 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !23459
  br label %__rustc::__rust_dealloc (.exit54)

__rustc::__rust_dealloc (.exit54): ; preds = %110, %128
  tail call void @free(ptr noundef nonnull %100) #92, !noalias !23459
  br label %130

130:                                              ; preds = %__rustc::__rust_dealloc (.exit54), %95
  %131 = getelementptr inbounds nuw i8, ptr %51, i64 96
  %132 = load ptr, ptr %131, align 8, !noundef !1733
  %133 = icmp eq ptr %132, null
  br i1 %133, label %1363, label %134

134:                                              ; preds = %130
  %135 = atomicrmw sub ptr %132, i64 1 release, align 8, !noalias !23460
  %136 = icmp eq i64 %135, 1
  br i1 %136, label %137, label %1363

137:                                              ; preds = %134
  fence acquire
; call <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %131) #91
  br label %1363

138:                                              ; preds = %52
  %139 = getelementptr inbounds nuw i8, ptr %46, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(96) %48, ptr noundef nonnull align 8 dereferenceable(96) %139, i64 96, i1 false)
; invoke <purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
  invoke fastcc void @<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>(ptr noalias nofree noundef align 8 captures(none) dereferenceable(32) %49, ptr noalias nofree noundef align 8 dereferenceable(104) %51, i64 noundef 0, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(96) %48)
          to label %140 unwind label %1368

140:                                              ; preds = %138
  %141 = load i64, ptr %49, align 8, !range !2052, !noundef !1733
  %142 = icmp eq i64 %141, -1
  br i1 %142, label %1328, label %143

143:                                              ; preds = %140
  call void @llvm.lifetime.start.p0(ptr nonnull %50)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %50, ptr noundef nonnull align 8 dereferenceable(32) %49, i64 32, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %48)
  call void @llvm.lifetime.end.p0(ptr nonnull %49)
  call void @llvm.lifetime.start.p0(ptr nonnull %47)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(104) %47, ptr noundef nonnull align 8 dereferenceable(104) %51, i64 104, i1 false)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !23467)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !23470)
  call void @llvm.lifetime.start.p0(ptr nonnull %45), !noalias !23472
  %144 = getelementptr inbounds nuw i8, ptr %50, i64 24
  %145 = load ptr, ptr %144, align 8, !alias.scope !23470, !noalias !23477, !nonnull !1733, !noundef !1733
  %146 = atomicrmw add ptr %145, i64 1 monotonic, align 8, !noalias !23472
  %147 = icmp slt i64 %146, 0
  br i1 %147, label %150, label %148

148:                                              ; preds = %143
  store ptr %145, ptr %45, align 8, !noalias !23472
; invoke <purrdf_sparql_eval::eval::EvalCtx>::may_fork_row_loop
  %149 = invoke fastcc noundef zeroext i1 @<purrdf_sparql_eval::eval::EvalCtx>::may_fork_row_loop(ptr noundef nonnull align 16 dereferenceable(1232) %4, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(64) %2)
          to label %157 unwind label %152, !noalias !23478

150:                                              ; preds = %143
  tail call void @llvm.trap()
  unreachable

151:                                              ; preds = %185
  br i1 %187, label %1316, label %1248

152:                                              ; preds = %1159, %178, %148
  %153 = phi i8 [ 1, %148 ], [ 1, %178 ], [ %1082, %1159 ]
  %154 = landingpad { ptr, i32 }
          cleanup
  br label %1316

155:                                              ; preds = %1085
  %156 = landingpad { ptr, i32 }
          cleanup
  br label %1248

157:                                              ; preds = %148
  %158 = getelementptr inbounds nuw i8, ptr %4, i64 472
  %159 = load i8, ptr %158, align 8, !range !3719
  %160 = icmp eq i8 %159, 2
  %161 = select i1 %149, i1 %160, i1 false
  br i1 %161, label %162, label %178

162:                                              ; preds = %157
  %163 = getelementptr inbounds nuw i8, ptr %4, i64 608
  %164 = load ptr, ptr %163, align 16, !noalias !23478, !noundef !1733
  %165 = icmp eq ptr %164, null
  br i1 %165, label %178, label %166

166:                                              ; preds = %162
  %167 = getelementptr inbounds nuw i8, ptr %164, i64 24
  %168 = load i64, ptr %167, align 8, !noalias !23479
  %169 = getelementptr inbounds nuw i8, ptr %164, i64 48
  %170 = icmp ult i64 %168, -2
  br i1 %170, label %178, label %171

171:                                              ; preds = %166
  %172 = getelementptr inbounds nuw i8, ptr %164, i64 32
  %173 = load i64, ptr %172, align 8, !noalias !23479
  %174 = icmp ult i64 %173, -2
  br i1 %174, label %178, label %175

175:                                              ; preds = %171
  %176 = load i64, ptr %169, align 8, !noalias !23479
  %177 = icmp ugt i64 %176, -3
  br label %178

178:                                              ; preds = %175, %171, %166, %162, %157
  %179 = phi i1 [ false, %157 ], [ false, %166 ], [ true, %162 ], [ false, %171 ], [ %177, %175 ]
  call void @llvm.lifetime.start.p0(ptr nonnull %44), !noalias !23472
  %180 = getelementptr inbounds nuw i8, ptr %50, i64 16
  %181 = load i64, ptr %180, align 8, !alias.scope !23470, !noalias !23477, !noundef !1733
  %182 = icmp ult i64 %181, 230584300921369396
  tail call void @llvm.assume(i1 %182)
; invoke <purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::for_rows::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::for_rows::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef align 8 captures(none) dereferenceable(200) %44, ptr noundef nonnull align 16 dereferenceable(1232) %4, i1 noundef zeroext %179, i64 noundef %181)
          to label %183 unwind label %152

183:                                              ; preds = %178
; invoke purrdf_sparql_eval::vm::program_at::<purrdf_core::ir::dataset::RdfDataset>
  %184 = invoke fastcc noundef nonnull ptr @purrdf_sparql_eval::vm::program_at::<purrdf_core::ir::dataset::RdfDataset>(ptr noundef nonnull align 16 dereferenceable(1232) %4, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %1, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(64) %2)
          to label %193 unwind label %189, !noalias !23478

185:                                              ; preds = %522, %189
  %186 = phi i8 [ %190, %189 ], [ %523, %522 ]
  %187 = phi i1 [ %191, %189 ], [ %524, %522 ]
  %188 = phi { ptr, i32 } [ %192, %189 ], [ %525, %522 ]
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(200) %44)
          to label %151 unwind label %543

189:                                              ; preds = %1083, %1081, %193, %183
  %190 = phi i8 [ %1082, %1081 ], [ %532, %1083 ], [ 1, %193 ], [ 1, %183 ]
  %191 = phi i1 [ true, %1081 ], [ false, %1083 ], [ true, %193 ], [ true, %183 ]
  %192 = landingpad { ptr, i32 }
          cleanup
  br label %185

193:                                              ; preds = %183
  call void @llvm.lifetime.start.p0(ptr nonnull %43), !noalias !23472
  %194 = load ptr, ptr %45, align 8, !noalias !23472, !nonnull !1733, !noundef !1733
  %195 = getelementptr inbounds nuw i8, ptr %194, i64 16
; invoke <purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::link::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::link::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef align 8 captures(address) dereferenceable(216) %43, ptr noundef nonnull %184, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(64) %2, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(56) %195, ptr noalias nofree noundef nonnull align 16 dereferenceable(1232) %4)
          to label %196 unwind label %189, !noalias !23478

196:                                              ; preds = %193
  call void @llvm.lifetime.start.p0(ptr nonnull %42), !noalias !23472
  br i1 %179, label %545, label %197

197:                                              ; preds = %196
  call void @llvm.lifetime.start.p0(ptr nonnull %24), !noalias !23472
  store i64 0, ptr %24, align 8, !noalias !23472
  %198 = getelementptr inbounds nuw i8, ptr %24, i64 8
  store ptr inttoptr (i64 8 to ptr), ptr %198, align 8, !noalias !23472
  %199 = getelementptr inbounds nuw i8, ptr %24, i64 16
  store i64 0, ptr %199, align 8, !noalias !23472
  %200 = getelementptr inbounds nuw i8, ptr %50, i64 8
  %201 = load ptr, ptr %200, align 8, !alias.scope !23470, !noalias !23477, !nonnull !1733, !noundef !1733
  %202 = load i64, ptr %50, align 8, !range !1828, !alias.scope !23470, !noalias !23477, !noundef !1733
  %203 = mul nuw nsw i64 %181, 40
  %204 = getelementptr inbounds nuw i8, ptr %201, i64 %203
  call void @llvm.lifetime.start.p0(ptr nonnull %23), !noalias !23472
  store ptr %201, ptr %23, align 8, !noalias !23472
  %205 = getelementptr inbounds nuw i8, ptr %23, i64 8
  %206 = getelementptr inbounds nuw i8, ptr %23, i64 16
  store i64 %202, ptr %206, align 8, !noalias !23472
  %207 = getelementptr inbounds nuw i8, ptr %23, i64 24
  store ptr %204, ptr %207, align 8, !noalias !23472
  call void @llvm.lifetime.start.p0(ptr nonnull %22)
  %208 = icmp eq i64 %181, 0
  br i1 %208, label %.loopexit81, label %209

209:                                              ; preds = %197
  %210 = getelementptr inbounds nuw i8, ptr %21, i64 8
  %211 = getelementptr inbounds nuw i8, ptr %21, i64 16
  %212 = getelementptr inbounds nuw i8, ptr %22, i64 8
  %213 = getelementptr inbounds nuw i8, ptr %9, i64 8
  %214 = getelementptr inbounds nuw i8, ptr %9, i64 12
  %215 = getelementptr inbounds nuw i8, ptr %8, i64 8
  %216 = tail call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %217 = tail call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  br label %225

218:                                              ; preds = %541, %417
  %219 = phi ptr [ %542, %541 ], [ %322, %417 ]
  %220 = phi { ptr, i32 } [ %539, %541 ], [ %415, %417 ]
  %221 = shl i64 %231, 3
  %222 = add i64 %221, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %219, i64 noundef %222, i64 noundef range(i64 1, -9223372036854775807) 4) #92, !noalias !1733
  br label %223

223:                                              ; preds = %538, %414, %218
  %224 = phi { ptr, i32 } [ %415, %414 ], [ %539, %538 ], [ %220, %218 ]
; call core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(32) %23) #89, !noalias !23478
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %24) #89, !noalias !23478
  br label %522

225:                                              ; preds = %418, %209
  %226 = phi ptr [ inttoptr (i64 8 to ptr), %209 ], [ %419, %418 ]
  %227 = phi i64 [ 0, %209 ], [ %420, %418 ]
  %228 = phi ptr [ inttoptr (i64 8 to ptr), %209 ], [ %421, %418 ]
  %229 = phi ptr [ %201, %209 ], [ %230, %418 ]
  %230 = getelementptr inbounds nuw i8, ptr %229, i64 40
  %231 = load i64, ptr %229, align 8, !noalias !23485
  %232 = getelementptr inbounds nuw i8, ptr %229, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %22, ptr noundef nonnull align 8 dereferenceable(32) %232, i64 32, i1 false), !noalias !23485
  %233 = icmp eq i64 %231, 0
  br i1 %233, label %.loopexit81, label %234

234:                                              ; preds = %225
  call void @llvm.lifetime.start.p0(ptr nonnull %21), !noalias !23472
  store i64 %231, ptr %21, align 8, !noalias !23472
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %210, ptr noundef nonnull align 8 dereferenceable(32) %22, i64 32, i1 false), !noalias !23472
  call void @llvm.lifetime.start.p0(ptr nonnull %20), !noalias !23472
; invoke <purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::pass::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::pass::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %20, ptr noalias nofree noundef align 8 dereferenceable(200) %44, ptr noundef nonnull align 16 dereferenceable(1232) %4)
          to label %316 unwind label %538, !noalias !23478

.loopexit81:                                      ; preds = %418, %225, %197
  %235 = phi ptr [ %201, %197 ], [ %204, %418 ], [ %230, %225 ]
  store ptr %235, ptr %205, align 8
  br label %236

236:                                              ; preds = %529, %.loopexit81
  %237 = phi ptr [ %230, %529 ], [ %235, %.loopexit81 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %22)
  %238 = ptrtoint ptr %204 to i64
  %239 = ptrtoint ptr %237 to i64
  %240 = sub nuw i64 %238, %239
  %241 = udiv exact i64 %240, 40
  call void @llvm.experimental.noalias.scope.decl(metadata !23488), !noalias !23478
  %242 = icmp eq ptr %204, %237
  br i1 %242, label %.loopexit76, label %.preheader75

.preheader75:                                     ; preds = %236
  %243 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %244 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  br label %245

245:                                              ; preds = %.preheader75, %283
  %246 = phi i64 [ %248, %283 ], [ 0, %.preheader75 ]
  %247 = getelementptr inbounds nuw [40 x i8], ptr %237, i64 %246
  %248 = add nuw nsw i64 %246, 1
  %249 = load i64, ptr %247, align 8, !range !1771, !alias.scope !23491, !noalias !23494, !noundef !1733
  %250 = icmp ugt i64 %249, 5
  br i1 %250, label %251, label %283

251:                                              ; preds = %245
  %252 = getelementptr i8, ptr %247, i64 8
  %253 = load ptr, ptr %252, align 8, !alias.scope !23488, !noalias !23494, !nonnull !1733, !noundef !1733
  %254 = shl i64 %249, 3
  %255 = add i64 %254, -8
  %256 = load i64, ptr %243, align 8, !noalias !23499, !noundef !1733
  %257 = call i64 @llvm.umin.i64(i64 %255, i64 9223372036854775807)
  %258 = call i64 @llvm.ssub.sat.i64(i64 %256, i64 %257)
  store i64 %258, ptr %243, align 8, !noalias !23499
  %259 = load i64, ptr %244, align 8, !noalias !23499, !noundef !1733
  %260 = icmp slt i64 %258, %259
  br i1 %260, label %261, label %.preheader431

261:                                              ; preds = %251
  store i64 %258, ptr %244, align 8, !noalias !23499
  br label %.preheader431

.preheader431:                                    ; preds = %261, %251
  br label %262

262:                                              ; preds = %.preheader431, %265
  %263 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !23499
  %264 = icmp slt i64 %263, 0
  br i1 %264, label %265, label %__rustc::__rust_dealloc (.exit55)

265:                                              ; preds = %262
  %266 = add nsw i64 %263, 1
  %267 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %263, i64 %266 acq_rel acquire, align 8, !noalias !23499
  %268 = extractvalue { i64, i1 } %267, 1
  br i1 %268, label %269, label %262

269:                                              ; preds = %265
  %270 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %257 monotonic, align 8, !noalias !23499
  %271 = call i64 @llvm.ssub.sat.i64(i64 %270, i64 %257)
  %272 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !23499
  br label %273

273:                                              ; preds = %276, %269
  %274 = phi i64 [ %272, %269 ], [ %279, %276 ]
  %275 = icmp slt i64 %271, %274
  br i1 %275, label %276, label %280

276:                                              ; preds = %273
  %277 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %274, i64 %271 monotonic monotonic, align 8, !noalias !23499
  %278 = extractvalue { i64, i1 } %277, 1
  %279 = extractvalue { i64, i1 } %277, 0
  br i1 %278, label %280, label %273

280:                                              ; preds = %276, %273
  %281 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !23499
  br label %__rustc::__rust_dealloc (.exit55)

__rustc::__rust_dealloc (.exit55): ; preds = %262, %280
  %282 = icmp ne i64 %255, 0
  call void @llvm.assume(i1 %282), !noalias !23499
  call void @free(ptr noundef nonnull %253) #92, !noalias !23499
  br label %283

283:                                              ; preds = %__rustc::__rust_dealloc (.exit55), %245
  %284 = icmp eq i64 %248, %241
  br i1 %284, label %.loopexit76, label %245

.loopexit76:                                      ; preds = %283, %236
  %285 = icmp eq i64 %202, 0
  br i1 %285, label %530, label %286

286:                                              ; preds = %.loopexit76
  %287 = mul nuw i64 %202, 40
  %288 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %289 = load i64, ptr %288, align 8, !noalias !23494, !noundef !1733
  %290 = call i64 @llvm.umin.i64(i64 %287, i64 9223372036854775807)
  %291 = call i64 @llvm.ssub.sat.i64(i64 %289, i64 %290)
  store i64 %291, ptr %288, align 8, !noalias !23494
  %292 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %293 = load i64, ptr %292, align 8, !noalias !23494, !noundef !1733
  %294 = icmp slt i64 %291, %293
  br i1 %294, label %295, label %.preheader430

295:                                              ; preds = %286
  store i64 %291, ptr %292, align 8, !noalias !23494
  br label %.preheader430

.preheader430:                                    ; preds = %295, %286
  br label %296

296:                                              ; preds = %.preheader430, %299
  %297 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !23494
  %298 = icmp slt i64 %297, 0
  br i1 %298, label %299, label %__rustc::__rust_dealloc (.exit56)

299:                                              ; preds = %296
  %300 = add nsw i64 %297, 1
  %301 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %297, i64 %300 acq_rel acquire, align 8, !noalias !23494
  %302 = extractvalue { i64, i1 } %301, 1
  br i1 %302, label %303, label %296

303:                                              ; preds = %299
  %304 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %290 monotonic, align 8, !noalias !23494
  %305 = call i64 @llvm.ssub.sat.i64(i64 %304, i64 %290)
  %306 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !23494
  br label %307

307:                                              ; preds = %310, %303
  %308 = phi i64 [ %306, %303 ], [ %313, %310 ]
  %309 = icmp slt i64 %305, %308
  br i1 %309, label %310, label %314

310:                                              ; preds = %307
  %311 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %308, i64 %305 monotonic monotonic, align 8, !noalias !23494
  %312 = extractvalue { i64, i1 } %311, 1
  %313 = extractvalue { i64, i1 } %311, 0
  br i1 %312, label %314, label %307

314:                                              ; preds = %310, %307
  %315 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !23494
  br label %__rustc::__rust_dealloc (.exit56)

__rustc::__rust_dealloc (.exit56): ; preds = %296, %314
  call void @free(ptr noundef nonnull %201) #92, !noalias !23494
  br label %530

316:                                              ; preds = %234
  %317 = load i8, ptr %20, align 8, !range !1736, !noalias !23472, !noundef !1733
  %318 = icmp eq i8 %317, -1
  br i1 %318, label %319, label %353

319:                                              ; preds = %316
  call void @llvm.lifetime.end.p0(ptr nonnull %20), !noalias !23472
  call void @llvm.lifetime.start.p0(ptr nonnull %19)
  %320 = add i64 %231, -1
  %321 = icmp ugt i64 %320, 4
  %322 = load ptr, ptr %210, align 8, !noalias !23472
  %323 = load i64, ptr %211, align 8, !noalias !23472
  %324 = add i64 %323, -1
  %325 = select i1 %321, i64 %324, i64 %320
  %326 = select i1 %321, ptr %322, ptr %210
  %327 = load ptr, ptr %45, align 8, !noalias !23472, !nonnull !1733, !noundef !1733
  %328 = getelementptr inbounds nuw i8, ptr %327, i64 16
  call void @llvm.lifetime.start.p0(ptr nonnull %9), !noalias !23502
; invoke <purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::term::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::term::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef align 16 captures(none) dereferenceable(96) %9, ptr noalias nofree noundef nonnull align 8 dereferenceable(216) %43, ptr noalias nofree noundef nonnull readonly align 4 captures(address, read_provenance) %326, i64 noundef range(i64 0, 1152921504606846976) %325, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(56) %328, ptr noalias nofree noundef nonnull align 16 dereferenceable(1232) %4)
          to label %329 unwind label %538, !inline_history !23509

329:                                              ; preds = %319
  %330 = load i64, ptr %9, align 16, !range !2520, !noalias !23502, !noundef !1733
  %331 = icmp eq i64 %330, -1
  %332 = load i32, ptr %213, align 8, !noalias !23502
  %333 = load i32, ptr %214, align 4, !noalias !23502
  br i1 %331, label %339, label %334

334:                                              ; preds = %329
  %335 = getelementptr inbounds nuw i8, ptr %9, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(80) %19, ptr noundef nonnull align 16 dereferenceable(80) %335, i64 80, i1 false), !noalias !23510
  call void @llvm.lifetime.end.p0(ptr nonnull %9), !noalias !23502
  %336 = trunc i32 %332 to i8
  %337 = lshr i32 %332, 8
  %338 = trunc nuw i32 %337 to i24
  br label %359

339:                                              ; preds = %329
  call void @llvm.lifetime.end.p0(ptr nonnull %9), !noalias !23502
  %340 = icmp eq i32 %332, 2
  br i1 %340, label %341, label %342

341:                                              ; preds = %339
  call void @llvm.lifetime.end.p0(ptr nonnull %19)
  br label %376

342:                                              ; preds = %339
  call void @llvm.lifetime.start.p0(ptr nonnull %8), !noalias !23502
; invoke purrdf_sparql_eval::expr::ebv_term::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @purrdf_sparql_eval::expr::ebv_term::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef align 16 captures(none) dereferenceable(96) %8, ptr noalias nofree noundef nonnull align 16 dereferenceable(1232) %4, i32 noundef %332, i32 noundef %333)
          to label %343 unwind label %538, !inline_history !23509

343:                                              ; preds = %342
  %344 = load i64, ptr %8, align 16, !range !2520, !noalias !23502, !noundef !1733
  %345 = icmp eq i64 %344, -1
  %346 = load i8, ptr %215, align 8, !noalias !23502
  br i1 %345, label %373, label %347

347:                                              ; preds = %343
  %348 = getelementptr inbounds nuw i8, ptr %8, i64 9
  %349 = load i24, ptr %348, align 1, !noalias !23510
  %350 = getelementptr inbounds nuw i8, ptr %8, i64 12
  %351 = load i32, ptr %350, align 4, !noalias !23510
  %352 = getelementptr inbounds nuw i8, ptr %8, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(80) %19, ptr noundef nonnull align 16 dereferenceable(80) %352, i64 80, i1 false), !noalias !23510
  call void @llvm.lifetime.end.p0(ptr nonnull %8), !noalias !23502
  br label %359

353:                                              ; preds = %316
  store ptr %230, ptr %205, align 8
  call void @llvm.lifetime.end.p0(ptr nonnull %20), !noalias !23472
  %354 = icmp ugt i64 %231, 5
  br i1 %354, label %355, label %529

355:                                              ; preds = %353
  %356 = load ptr, ptr %210, align 8, !nonnull !1733, !noundef !1733
  %357 = shl i64 %231, 3
  %358 = add i64 %357, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %356, i64 noundef %358, i64 noundef range(i64 1, -9223372036854775807) 4) #92, !noalias !23511
  br label %529

359:                                              ; preds = %347, %334
  %360 = phi i24 [ %338, %334 ], [ %349, %347 ]
  %361 = phi i8 [ %336, %334 ], [ %346, %347 ]
  %362 = phi i32 [ %333, %334 ], [ %351, %347 ]
  %363 = phi i64 [ %330, %334 ], [ %344, %347 ]
  %364 = getelementptr inbounds nuw i8, ptr %0, i64 25
  store i24 %360, ptr %364, align 1, !noalias !23514
  %365 = getelementptr inbounds nuw i8, ptr %0, i64 28
  store i32 %362, ptr %365, align 4, !noalias !23514
  %366 = getelementptr inbounds nuw i8, ptr %0, i64 32
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(80) %366, ptr noundef nonnull align 16 dereferenceable(80) %19, i64 80, i1 false), !noalias !23514
  call void @llvm.lifetime.end.p0(ptr nonnull %19)
  %367 = getelementptr inbounds nuw i8, ptr %0, i64 16
  store i64 %363, ptr %367, align 16, !alias.scope !23467, !noalias !23514
  %368 = getelementptr inbounds nuw i8, ptr %0, i64 24
  store i8 %361, ptr %368, align 8, !alias.scope !23467, !noalias !23514
  store i64 1, ptr %0, align 16, !alias.scope !23467, !noalias !23514
  %369 = icmp ugt i64 %231, 5
  br i1 %369, label %370, label %429

370:                                              ; preds = %359
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %322) ]
  %371 = shl i64 %231, 3
  %372 = add i64 %371, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %322, i64 noundef %372, i64 noundef range(i64 1, -9223372036854775807) 4) #92, !noalias !23515
  br label %429

373:                                              ; preds = %343
  call void @llvm.lifetime.end.p0(ptr nonnull %8), !noalias !23502
  call void @llvm.lifetime.end.p0(ptr nonnull %19)
  %374 = and i8 %346, 1
  %375 = icmp eq i8 %374, 0
  br i1 %375, label %376, label %408

376:                                              ; preds = %373, %341
  %377 = icmp ugt i64 %231, 5
  br i1 %377, label %378, label %418

378:                                              ; preds = %376
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %322) ]
  %379 = shl i64 %231, 3
  %380 = add i64 %379, -8
  %381 = load i64, ptr %216, align 8, !noalias !23518, !noundef !1733
  %382 = call i64 @llvm.umin.i64(i64 %380, i64 9223372036854775807)
  %383 = call i64 @llvm.ssub.sat.i64(i64 %381, i64 %382)
  store i64 %383, ptr %216, align 8, !noalias !23518
  %384 = load i64, ptr %217, align 8, !noalias !23518, !noundef !1733
  %385 = icmp slt i64 %383, %384
  br i1 %385, label %386, label %.preheader434

386:                                              ; preds = %378
  store i64 %383, ptr %217, align 8, !noalias !23518
  br label %.preheader434

.preheader434:                                    ; preds = %386, %378
  br label %387

387:                                              ; preds = %.preheader434, %390
  %388 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !23518
  %389 = icmp slt i64 %388, 0
  br i1 %389, label %390, label %__rustc::__rust_dealloc (.exit57)

390:                                              ; preds = %387
  %391 = add nsw i64 %388, 1
  %392 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %388, i64 %391 acq_rel acquire, align 8, !noalias !23518
  %393 = extractvalue { i64, i1 } %392, 1
  br i1 %393, label %394, label %387

394:                                              ; preds = %390
  %395 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %382 monotonic, align 8, !noalias !23518
  %396 = call i64 @llvm.ssub.sat.i64(i64 %395, i64 %382)
  %397 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !23518
  br label %398

398:                                              ; preds = %401, %394
  %399 = phi i64 [ %397, %394 ], [ %404, %401 ]
  %400 = icmp slt i64 %396, %399
  br i1 %400, label %401, label %405

401:                                              ; preds = %398
  %402 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %399, i64 %396 monotonic monotonic, align 8, !noalias !23518
  %403 = extractvalue { i64, i1 } %402, 1
  %404 = extractvalue { i64, i1 } %402, 0
  br i1 %403, label %405, label %398

405:                                              ; preds = %401, %398
  %406 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !23518
  br label %__rustc::__rust_dealloc (.exit57)

__rustc::__rust_dealloc (.exit57): ; preds = %387, %405
  %407 = icmp ne i64 %380, 0
  call void @llvm.assume(i1 %407), !noalias !23518
  call void @free(ptr noundef nonnull %322) #92, !noalias !23518
  br label %418

408:                                              ; preds = %373
  call void @llvm.lifetime.start.p0(ptr nonnull %18)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %18, ptr noundef nonnull align 8 dereferenceable(24) %212, i64 24, i1 false)
  call void @llvm.experimental.noalias.scope.decl(metadata !23521)
  %409 = load i64, ptr %24, align 8, !range !1828, !alias.scope !23521, !noalias !23524, !noundef !1733
  %410 = icmp eq i64 %227, %409
  br i1 %410, label %411, label %423

411:                                              ; preds = %408
; invoke <alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one
  invoke void @<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %24)
          to label %412 unwind label %414, !noalias !23524

412:                                              ; preds = %411
  %413 = load ptr, ptr %198, align 8, !alias.scope !23521, !noalias !23524
  br label %423

414:                                              ; preds = %411
  %415 = landingpad { ptr, i32 }
          cleanup
  store ptr %230, ptr %205, align 8
  %416 = icmp ugt i64 %231, 5
  br i1 %416, label %417, label %223

417:                                              ; preds = %414
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %322) ]
  br label %218

418:                                              ; preds = %423, %__rustc::__rust_dealloc (.exit57), %376
  %419 = phi ptr [ %226, %__rustc::__rust_dealloc (.exit57) ], [ %226, %376 ], [ %424, %423 ]
  %420 = phi i64 [ %227, %__rustc::__rust_dealloc (.exit57) ], [ %227, %376 ], [ %428, %423 ]
  %421 = phi ptr [ %228, %__rustc::__rust_dealloc (.exit57) ], [ %228, %376 ], [ %424, %423 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %21), !noalias !23472
  call void @llvm.lifetime.end.p0(ptr nonnull %22)
  call void @llvm.lifetime.start.p0(ptr nonnull %22)
  %422 = icmp eq ptr %230, %204
  br i1 %422, label %.loopexit81, label %225

423:                                              ; preds = %412, %408
  %424 = phi ptr [ %413, %412 ], [ %226, %408 ]
  %425 = getelementptr inbounds nuw [40 x i8], ptr %424, i64 %227
  store i64 %231, ptr %425, align 8, !noalias !23526
  %426 = getelementptr inbounds nuw i8, ptr %425, i64 8
  store ptr %322, ptr %426, align 8, !noalias !23526
  %427 = getelementptr inbounds nuw i8, ptr %425, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %427, ptr noundef nonnull align 8 dereferenceable(24) %18, i64 24, i1 false), !noalias !23526
  %428 = add i64 %227, 1
  store i64 %428, ptr %199, align 8, !alias.scope !23521, !noalias !23524
  call void @llvm.lifetime.end.p0(ptr nonnull %18)
  br label %418

429:                                              ; preds = %370, %359
  call void @llvm.lifetime.end.p0(ptr nonnull %21), !noalias !23472
  call void @llvm.lifetime.end.p0(ptr nonnull %22)
  %430 = ptrtoint ptr %204 to i64
  %431 = ptrtoint ptr %230 to i64
  %432 = sub nuw i64 %430, %431
  %433 = udiv exact i64 %432, 40
  call void @llvm.experimental.noalias.scope.decl(metadata !23527), !noalias !23478
  %434 = icmp eq ptr %204, %230
  br i1 %434, label %.loopexit80, label %.preheader79

.preheader79:                                     ; preds = %429, %472
  %435 = phi i64 [ %437, %472 ], [ 0, %429 ]
  %436 = getelementptr inbounds nuw [40 x i8], ptr %230, i64 %435
  %437 = add nuw nsw i64 %435, 1
  %438 = load i64, ptr %436, align 8, !range !1771, !alias.scope !23530, !noalias !23533, !noundef !1733
  %439 = icmp ugt i64 %438, 5
  br i1 %439, label %440, label %472

440:                                              ; preds = %.preheader79
  %441 = getelementptr i8, ptr %436, i64 8
  %442 = load ptr, ptr %441, align 8, !alias.scope !23527, !noalias !23533, !nonnull !1733, !noundef !1733
  %443 = shl i64 %438, 3
  %444 = add i64 %443, -8
  %445 = load i64, ptr %216, align 8, !noalias !23538, !noundef !1733
  %446 = call i64 @llvm.umin.i64(i64 %444, i64 9223372036854775807)
  %447 = call i64 @llvm.ssub.sat.i64(i64 %445, i64 %446)
  store i64 %447, ptr %216, align 8, !noalias !23538
  %448 = load i64, ptr %217, align 8, !noalias !23538, !noundef !1733
  %449 = icmp slt i64 %447, %448
  br i1 %449, label %450, label %.preheader433

450:                                              ; preds = %440
  store i64 %447, ptr %217, align 8, !noalias !23538
  br label %.preheader433

.preheader433:                                    ; preds = %450, %440
  br label %451

451:                                              ; preds = %.preheader433, %454
  %452 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !23538
  %453 = icmp slt i64 %452, 0
  br i1 %453, label %454, label %__rustc::__rust_dealloc (.exit58)

454:                                              ; preds = %451
  %455 = add nsw i64 %452, 1
  %456 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %452, i64 %455 acq_rel acquire, align 8, !noalias !23538
  %457 = extractvalue { i64, i1 } %456, 1
  br i1 %457, label %458, label %451

458:                                              ; preds = %454
  %459 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %446 monotonic, align 8, !noalias !23538
  %460 = call i64 @llvm.ssub.sat.i64(i64 %459, i64 %446)
  %461 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !23538
  br label %462

462:                                              ; preds = %465, %458
  %463 = phi i64 [ %461, %458 ], [ %468, %465 ]
  %464 = icmp slt i64 %460, %463
  br i1 %464, label %465, label %469

465:                                              ; preds = %462
  %466 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %463, i64 %460 monotonic monotonic, align 8, !noalias !23538
  %467 = extractvalue { i64, i1 } %466, 1
  %468 = extractvalue { i64, i1 } %466, 0
  br i1 %467, label %469, label %462

469:                                              ; preds = %465, %462
  %470 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !23538
  br label %__rustc::__rust_dealloc (.exit58)

__rustc::__rust_dealloc (.exit58): ; preds = %451, %469
  %471 = icmp ne i64 %444, 0
  call void @llvm.assume(i1 %471), !noalias !23538
  call void @free(ptr noundef nonnull %442) #92, !noalias !23538
  br label %472

472:                                              ; preds = %__rustc::__rust_dealloc (.exit58), %.preheader79
  %473 = icmp eq i64 %437, %433
  br i1 %473, label %.loopexit80, label %.preheader79

.loopexit80:                                      ; preds = %472, %429
  %474 = icmp eq i64 %202, 0
  br i1 %474, label %477, label %475

475:                                              ; preds = %.loopexit80
  %476 = mul nuw i64 %202, 40
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %201, i64 noundef %476, i64 noundef range(i64 1, -9223372036854775807) 8) #92, !noalias !23533
  br label %477

477:                                              ; preds = %475, %.loopexit80
  call void @llvm.lifetime.end.p0(ptr nonnull %23), !noalias !23472
  call void @llvm.experimental.noalias.scope.decl(metadata !23541)
  call void @llvm.experimental.noalias.scope.decl(metadata !23544), !noalias !23478
  %478 = icmp eq i64 %227, 0
  br i1 %478, label %.loopexit78, label %.preheader77

.preheader77:                                     ; preds = %477, %516
  %479 = phi i64 [ %481, %516 ], [ 0, %477 ]
  %480 = getelementptr inbounds nuw [40 x i8], ptr %228, i64 %479
  %481 = add nuw nsw i64 %479, 1
  %482 = load i64, ptr %480, align 8, !range !1771, !alias.scope !23547, !noalias !23550, !noundef !1733
  %483 = icmp ugt i64 %482, 5
  br i1 %483, label %484, label %516

484:                                              ; preds = %.preheader77
  %485 = getelementptr i8, ptr %480, i64 8
  %486 = load ptr, ptr %485, align 8, !alias.scope !23544, !noalias !23550, !nonnull !1733, !noundef !1733
  %487 = shl i64 %482, 3
  %488 = add i64 %487, -8
  %489 = load i64, ptr %216, align 8, !noalias !23551, !noundef !1733
  %490 = call i64 @llvm.umin.i64(i64 %488, i64 9223372036854775807)
  %491 = call i64 @llvm.ssub.sat.i64(i64 %489, i64 %490)
  store i64 %491, ptr %216, align 8, !noalias !23551
  %492 = load i64, ptr %217, align 8, !noalias !23551, !noundef !1733
  %493 = icmp slt i64 %491, %492
  br i1 %493, label %494, label %.preheader432

494:                                              ; preds = %484
  store i64 %491, ptr %217, align 8, !noalias !23551
  br label %.preheader432

.preheader432:                                    ; preds = %494, %484
  br label %495

495:                                              ; preds = %.preheader432, %498
  %496 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !23551
  %497 = icmp slt i64 %496, 0
  br i1 %497, label %498, label %__rustc::__rust_dealloc (.exit59)

498:                                              ; preds = %495
  %499 = add nsw i64 %496, 1
  %500 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %496, i64 %499 acq_rel acquire, align 8, !noalias !23551
  %501 = extractvalue { i64, i1 } %500, 1
  br i1 %501, label %502, label %495

502:                                              ; preds = %498
  %503 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %490 monotonic, align 8, !noalias !23551
  %504 = call i64 @llvm.ssub.sat.i64(i64 %503, i64 %490)
  %505 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !23551
  br label %506

506:                                              ; preds = %509, %502
  %507 = phi i64 [ %505, %502 ], [ %512, %509 ]
  %508 = icmp slt i64 %504, %507
  br i1 %508, label %509, label %513

509:                                              ; preds = %506
  %510 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %507, i64 %504 monotonic monotonic, align 8, !noalias !23551
  %511 = extractvalue { i64, i1 } %510, 1
  %512 = extractvalue { i64, i1 } %510, 0
  br i1 %511, label %513, label %506

513:                                              ; preds = %509, %506
  %514 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !23551
  br label %__rustc::__rust_dealloc (.exit59)

__rustc::__rust_dealloc (.exit59): ; preds = %495, %513
  %515 = icmp ne i64 %488, 0
  call void @llvm.assume(i1 %515), !noalias !23551
  call void @free(ptr noundef nonnull %486) #92, !noalias !23551
  br label %516

516:                                              ; preds = %__rustc::__rust_dealloc (.exit59), %.preheader77
  %517 = icmp eq i64 %481, %227
  br i1 %517, label %.loopexit78, label %.preheader77

.loopexit78:                                      ; preds = %516, %477
  %518 = load i64, ptr %24, align 8, !alias.scope !23541, !noalias !23478
  %519 = icmp eq i64 %518, 0
  br i1 %519, label %528, label %520

520:                                              ; preds = %.loopexit78
  %521 = mul nuw i64 %518, 40
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %228, i64 noundef %521, i64 noundef range(i64 1, -9223372036854775807) 8) #92, !noalias !23550
  br label %528

522:                                              ; preds = %1145, %931, %571, %568, %564, %526, %223
  %523 = phi i8 [ 1, %526 ], [ 0, %223 ], [ %532, %1145 ], [ %532, %931 ], [ 1, %571 ], [ 1, %564 ], [ 1, %568 ]
  %524 = phi i1 [ true, %526 ], [ true, %223 ], [ true, %1145 ], [ false, %931 ], [ true, %571 ], [ true, %564 ], [ true, %568 ]
  %525 = phi { ptr, i32 } [ %527, %526 ], [ %224, %223 ], [ %1146, %1145 ], [ %932, %931 ], [ %565, %571 ], [ %565, %564 ], [ %565, %568 ]
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>(ptr noalias nofree noundef align 8 dereferenceable(216) %43) #89
          to label %185 unwind label %543, !noalias !23478

526:                                              ; preds = %1153, %683, %545
  %527 = landingpad { ptr, i32 }
          cleanup
  br label %522

528:                                              ; preds = %520, %.loopexit78
  call void @llvm.lifetime.end.p0(ptr nonnull %24), !noalias !23472
  br label %1081

529:                                              ; preds = %355, %353
  call void @llvm.lifetime.end.p0(ptr nonnull %21), !noalias !23472
  br label %236

530:                                              ; preds = %__rustc::__rust_dealloc (.exit56), %.loopexit76
  call void @llvm.lifetime.end.p0(ptr nonnull %23), !noalias !23472
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %42, ptr noundef nonnull align 8 dereferenceable(24) %24, i64 24, i1 false), !noalias !23472
  call void @llvm.lifetime.end.p0(ptr nonnull %24), !noalias !23472
  br label %531

531:                                              ; preds = %930, %530
  %532 = phi i8 [ 1, %930 ], [ 0, %530 ]
  %533 = getelementptr inbounds nuw i8, ptr %4, i64 688
  call void @llvm.experimental.noalias.scope.decl(metadata !23554)
  %534 = load ptr, ptr %533, align 16, !alias.scope !23554, !noalias !23557, !nonnull !1733, !noundef !1733
  %535 = getelementptr inbounds nuw i8, ptr %534, i64 40
  %536 = load atomic i32, ptr %535 acquire, align 4, !noalias !23559
  %537 = icmp eq i32 %536, 0
  br i1 %537, label %933, label %943

538:                                              ; preds = %342, %319, %234
  %539 = landingpad { ptr, i32 }
          cleanup
  store ptr %230, ptr %205, align 8
  %540 = icmp ugt i64 %231, 5
  br i1 %540, label %541, label %223

541:                                              ; preds = %538
  %542 = load ptr, ptr %210, align 8, !nonnull !1733, !noundef !1733
  br label %218

543:                                              ; preds = %1323, %1322, %1254, %1154, %707, %571, %522, %185
  %544 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #90, !noalias !23467
  unreachable

545:                                              ; preds = %196
  %546 = getelementptr inbounds nuw i8, ptr %44, i64 184
  %547 = load i64, ptr %546, align 8, !noundef !1733
  %548 = tail call noundef range(i64 0, 230584300921369396) i64 @llvm.umin.i64(i64 %547, i64 range(i64 0, 230584300921369396) %181)
  %549 = getelementptr inbounds nuw i8, ptr %50, i64 8
  %550 = load ptr, ptr %549, align 8, !alias.scope !23470, !noalias !23477, !nonnull !1733, !noundef !1733
  call void @llvm.lifetime.start.p0(ptr nonnull %41), !noalias !23472
; invoke <purrdf_sparql_eval::eval::EvalCtx>::loop_snapshot
  %551 = invoke fastcc noundef ptr @<purrdf_sparql_eval::eval::EvalCtx>::loop_snapshot(ptr noundef nonnull align 16 dereferenceable(1232) %4, i64 noundef %548)
          to label %552 unwind label %526, !noalias !23478

552:                                              ; preds = %545
  store ptr %551, ptr %41, align 8, !noalias !23472
  call void @llvm.lifetime.start.p0(ptr nonnull %40)
  call void @llvm.lifetime.start.p0(ptr nonnull %39), !noalias !23472
  %553 = getelementptr inbounds nuw i8, ptr %4, i64 608
  %554 = load ptr, ptr %553, align 16, !noundef !1733
  %555 = icmp eq ptr %554, null
  br i1 %555, label %574, label %556

556:                                              ; preds = %552
  %557 = getelementptr inbounds nuw i8, ptr %554, i64 16
  %558 = load i64, ptr %557, align 8
  %559 = icmp ugt i64 %558, -3
  br i1 %559, label %560, label %574

560:                                              ; preds = %556
  %561 = getelementptr inbounds nuw i8, ptr %554, i64 40
  %562 = load i64, ptr %561, align 8
  %563 = icmp ult i64 %562, -2
  br label %574

564:                                              ; preds = %1156, %1154, %669, %617, %572
  %565 = phi { ptr, i32 } [ %1157, %1156 ], [ %618, %617 ], [ %573, %572 ], [ %1155, %1154 ], [ %670, %669 ]
  call void @llvm.experimental.noalias.scope.decl(metadata !23560)
  %566 = load ptr, ptr %41, align 8, !alias.scope !23560, !noalias !23478, !noundef !1733
  %567 = icmp eq ptr %566, null
  br i1 %567, label %522, label %568

568:                                              ; preds = %564
  %569 = atomicrmw sub ptr %566, i64 1 release, align 8, !noalias !23563
  %570 = icmp eq i64 %569, 1
  br i1 %570, label %571, label %522

571:                                              ; preds = %568
  fence acquire, !noalias !23478
; invoke <alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %41) #91
          to label %522 unwind label %543, !inline_history !2018

572:                                              ; preds = %585, %584
  %573 = landingpad { ptr, i32 }
          cleanup
  br label %564

574:                                              ; preds = %560, %556, %552
  %575 = phi i1 [ false, %552 ], [ true, %556 ], [ %563, %560 ]
  %576 = getelementptr inbounds nuw i8, ptr %4, i64 1226
  %577 = load i8, ptr %576, align 2, !range !1740, !noundef !1733
  %578 = trunc nuw i8 %577 to i1
  call void @llvm.lifetime.start.p0(ptr nonnull %38), !noalias !23472
  store ptr %4, ptr %38, align 8, !noalias !23472
  %579 = getelementptr inbounds nuw i8, ptr %38, i64 8
  store ptr %41, ptr %579, align 8, !noalias !23472
  %580 = getelementptr inbounds nuw i8, ptr %38, i64 16
  store ptr %44, ptr %580, align 8, !noalias !23472
  %581 = getelementptr inbounds nuw i8, ptr %38, i64 24
  store ptr %43, ptr %581, align 8, !noalias !23472
  call void @llvm.lifetime.start.p0(ptr nonnull %37), !noalias !23472
  store ptr %550, ptr %37, align 8, !noalias !23472
  %582 = getelementptr inbounds nuw i8, ptr %37, i64 8
  store i64 %548, ptr %582, align 8, !noalias !23472
  %583 = getelementptr inbounds nuw i8, ptr %37, i64 16
  store ptr %45, ptr %583, align 8, !noalias !23472
  br i1 %575, label %585, label %584

584:                                              ; preds = %574
; invoke purrdf_sparql_eval::parallel::par_chunk_try_map_init::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_filter_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#0}, purrdf_sparql_eval::expr::eval_filter_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#1}, purrdf_sparql_eval::expr::eval_filter_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#2}>
  invoke fastcc void @purrdf_sparql_eval::parallel::par_chunk_try_map_init::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_filter_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#0}, purrdf_sparql_eval::expr::eval_filter_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#1}, purrdf_sparql_eval::expr::eval_filter_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#2}>(ptr noalias nofree noundef nonnull align 16 captures(address) dereferenceable(256) %39, i1 noundef zeroext %578, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) %550, i64 noundef range(i64 0, 230584300921369396) %548, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(32) %38, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(24) %37, ptr noundef nonnull align 8 %44)
          to label %586 unwind label %572, !inline_history !23566

585:                                              ; preds = %574
; invoke purrdf_sparql_eval::parallel::par_blocks_try_map_init::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_filter_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#0}, purrdf_sparql_eval::expr::eval_filter_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#1}, purrdf_sparql_eval::expr::eval_filter_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#2}>
  invoke fastcc void @purrdf_sparql_eval::parallel::par_blocks_try_map_init::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_filter_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#0}, purrdf_sparql_eval::expr::eval_filter_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#1}, purrdf_sparql_eval::expr::eval_filter_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#2}>(ptr noalias nofree noundef nonnull align 16 captures(address) dereferenceable(256) %39, i1 noundef zeroext %578, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) %550, i64 noundef range(i64 0, 230584300921369396) %548, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(32) %38, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(24) %37, ptr noundef nonnull align 8 %44)
          to label %586 unwind label %572, !inline_history !23566

586:                                              ; preds = %585, %584
  call void @llvm.lifetime.end.p0(ptr nonnull %37), !noalias !23472
  call void @llvm.lifetime.end.p0(ptr nonnull %38), !noalias !23472
  %587 = load i64, ptr %39, align 16, !range !2052, !noalias !23472, !noundef !1733
  %588 = icmp eq i64 %587, -1
  br i1 %588, label %589, label %595

589:                                              ; preds = %586
  %590 = getelementptr inbounds nuw i8, ptr %39, i64 16
  %591 = getelementptr inbounds nuw i8, ptr %39, i64 48
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(64) %40, ptr noundef nonnull align 16 dereferenceable(64) %591, i64 64, i1 false), !noalias !23472
  %592 = getelementptr inbounds nuw i8, ptr %0, i64 48
  %593 = getelementptr inbounds nuw i8, ptr %0, i64 16
  %594 = load <4 x i64>, ptr %590, align 16, !noalias !23472
  call void @llvm.lifetime.end.p0(ptr nonnull %39), !noalias !23472
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(64) %592, ptr noundef nonnull align 16 dereferenceable(64) %40, i64 64, i1 false), !noalias !23514
  store <4 x i64> %594, ptr %593, align 16, !alias.scope !23467, !noalias !23514
  store i64 1, ptr %0, align 16, !alias.scope !23467, !noalias !23514
  call void @llvm.lifetime.end.p0(ptr nonnull %40)
  br label %1147

595:                                              ; preds = %586
  %596 = getelementptr inbounds nuw i8, ptr %39, i64 8
  %597 = getelementptr inbounds nuw i8, ptr %39, i64 24
  %598 = load i64, ptr %597, align 8, !noalias !23472
  %599 = getelementptr inbounds nuw i8, ptr %39, i64 32
  %600 = load i64, ptr %599, align 16, !noalias !23472
  %601 = getelementptr inbounds nuw i8, ptr %39, i64 40
  %602 = load i64, ptr %601, align 8, !noalias !23472
  %603 = getelementptr inbounds nuw i8, ptr %39, i64 48
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(208) %40, ptr noundef nonnull align 16 dereferenceable(208) %603, i64 208, i1 false), !noalias !23472
  %604 = getelementptr inbounds nuw i8, ptr %33, i64 24
  %605 = getelementptr inbounds nuw i8, ptr %29, i64 8
  %606 = load <2 x i64>, ptr %596, align 8, !noalias !23472
  call void @llvm.lifetime.end.p0(ptr nonnull %39), !noalias !23472
  call void @llvm.lifetime.start.p0(ptr nonnull %33), !noalias !23472
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(208) %604, ptr noundef nonnull align 16 dereferenceable(208) %40, i64 208, i1 false), !noalias !23472
  store i64 %587, ptr %29, align 8
  store <2 x i64> %606, ptr %605, align 8
  call void @llvm.lifetime.end.p0(ptr nonnull %40)
  call void @llvm.lifetime.start.p0(ptr nonnull %34), !noalias !23472
  %607 = add i64 %598, -3
  %608 = icmp ult i64 %607, -2
  %609 = select i1 %608, i64 %602, i64 %598
  %610 = add i64 %609, -1
  %611 = select i1 %608, i64 %598, i64 1
  %612 = select i1 %608, i64 1, i64 %602
  store i64 %611, ptr %33, align 8, !noalias !23472
  %613 = getelementptr inbounds nuw i8, ptr %33, i64 8
  store i64 %600, ptr %613, align 8, !noalias !23472
  %614 = getelementptr inbounds nuw i8, ptr %33, i64 16
  store i64 %612, ptr %614, align 8, !noalias !23472
  %615 = getelementptr inbounds nuw i8, ptr %33, i64 232
  store i64 0, ptr %615, align 8, !noalias !23472
  %616 = getelementptr inbounds nuw i8, ptr %33, i64 240
  store i64 %610, ptr %616, align 8, !noalias !23472
; invoke <purrdf_core::small::IntoIter<[(purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint); 1]> as core::iter::traits::iterator::Iterator>::unzip::<purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>
  invoke fastcc void @<purrdf_core::small::IntoIter<[(purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint); 1]> as core::iter::traits::iterator::Iterator>::unzip::<purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>(ptr noalias nofree noundef align 8 captures(none) dereferenceable(240) %34, ptr noalias nofree noundef align 8 captures(address) dereferenceable(248) %33)
          to label %619 unwind label %1156, !noalias !23478

617:                                              ; preds = %629
  %618 = landingpad { ptr, i32 }
          cleanup
  br label %564

619:                                              ; preds = %595
  call void @llvm.lifetime.end.p0(ptr nonnull %33), !noalias !23472
  call void @llvm.lifetime.start.p0(ptr nonnull %36), !noalias !23472
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %36, ptr noundef nonnull align 8 dereferenceable(32) %34, i64 32, i1 false), !noalias !23472
  call void @llvm.lifetime.start.p0(ptr nonnull %35), !noalias !23472
  %620 = getelementptr inbounds nuw i8, ptr %34, i64 32
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(208) %35, ptr noundef nonnull align 8 dereferenceable(208) %620, i64 208, i1 false), !noalias !23472
  call void @llvm.lifetime.end.p0(ptr nonnull %34), !noalias !23472
  call void @llvm.lifetime.start.p0(ptr nonnull %31)
  call void @llvm.lifetime.start.p0(ptr nonnull %30), !noalias !23472
; invoke <purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::commit_rows::<purrdf_core::ir::dataset::RdfDataset, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>
  invoke fastcc void @<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::commit_rows::<purrdf_core::ir::dataset::RdfDataset, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>(ptr noalias nofree noundef align 16 captures(address) dereferenceable(96) %30, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(200) %44, ptr noalias nofree noundef nonnull align 16 dereferenceable(1232) %4, ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %29, ptr noalias nofree noundef align 8 captures(address) dereferenceable(208) %35)
          to label %621 unwind label %1154

621:                                              ; preds = %619
  %622 = load i64, ptr %30, align 16, !range !2520, !noalias !23472, !noundef !1733
  %623 = icmp eq i64 %622, -1
  %624 = getelementptr inbounds nuw i8, ptr %30, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %31, ptr noundef nonnull align 8 dereferenceable(24) %624, i64 24, i1 false), !noalias !23472
  %625 = getelementptr inbounds nuw i8, ptr %30, i64 32
  %626 = getelementptr inbounds nuw i8, ptr %30, i64 40
  %627 = load i64, ptr %626, align 8, !noalias !23472
  %628 = load i64, ptr %625, align 16, !noalias !23472
  br i1 %623, label %636, label %629

629:                                              ; preds = %621
  %630 = getelementptr inbounds nuw i8, ptr %30, i64 48
  %631 = getelementptr inbounds nuw i8, ptr %0, i64 64
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(48) %631, ptr noundef nonnull align 16 dereferenceable(48) %630, i64 48, i1 false), !noalias !23514
  call void @llvm.lifetime.end.p0(ptr nonnull %30), !noalias !23472
  %632 = getelementptr inbounds nuw i8, ptr %0, i64 24
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %632, ptr noundef nonnull align 8 dereferenceable(24) %31, i64 24, i1 false), !noalias !23514
  %633 = getelementptr inbounds nuw i8, ptr %0, i64 16
  store i64 %622, ptr %633, align 16, !alias.scope !23467, !noalias !23514
  %634 = getelementptr inbounds nuw i8, ptr %0, i64 48
  store i64 %628, ptr %634, align 16, !alias.scope !23467, !noalias !23514
  %635 = getelementptr inbounds nuw i8, ptr %0, i64 56
  store i64 %627, ptr %635, align 8, !alias.scope !23467, !noalias !23514
  store i64 1, ptr %0, align 16, !alias.scope !23467, !noalias !23514
  call void @llvm.lifetime.end.p0(ptr nonnull %31)
  call void @llvm.lifetime.end.p0(ptr nonnull %35), !noalias !23472
; invoke core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>>(ptr noalias nofree noundef align 8 dereferenceable(32) %36)
          to label %927 unwind label %617, !noalias !23478

636:                                              ; preds = %621
  call void @llvm.lifetime.end.p0(ptr nonnull %30), !noalias !23472
  call void @llvm.lifetime.start.p0(ptr nonnull %32), !noalias !23472
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %32, ptr noundef nonnull align 8 dereferenceable(24) %31, i64 24, i1 false), !noalias !23472
  call void @llvm.lifetime.end.p0(ptr nonnull %31)
  %637 = load i64, ptr %36, align 8, !noalias !23472
  %638 = getelementptr inbounds nuw i8, ptr %36, i64 8
  %639 = load i64, ptr %638, align 8, !noalias !23472
  %640 = getelementptr inbounds nuw i8, ptr %36, i64 16
  %641 = load i64, ptr %640, align 8, !noalias !23472
  %642 = getelementptr inbounds nuw i8, ptr %36, i64 24
  %643 = load i64, ptr %642, align 8, !noalias !23472
  %644 = icmp ugt i64 %637, 2
  %645 = select i1 %644, i64 %641, i64 %637
  %646 = add i64 %645, -1
  %647 = select i1 %644, i64 %637, i64 1
  %648 = select i1 %644, i64 1, i64 %641
  call void @llvm.lifetime.start.p0(ptr nonnull %12), !noalias !23567
  store i64 %647, ptr %12, align 8, !noalias !23571
  %649 = getelementptr inbounds nuw i8, ptr %12, i64 8
  store i64 %639, ptr %649, align 8, !noalias !23571
  %650 = getelementptr inbounds nuw i8, ptr %12, i64 16
  store i64 %648, ptr %650, align 8, !noalias !23571
  %651 = getelementptr inbounds nuw i8, ptr %12, i64 24
  store i64 %643, ptr %651, align 8, !noalias !23571
  %652 = getelementptr inbounds nuw i8, ptr %12, i64 32
  store i64 0, ptr %652, align 8, !noalias !23567
  %653 = getelementptr inbounds nuw i8, ptr %12, i64 40
  store i64 %646, ptr %653, align 8, !noalias !23567
  %654 = icmp eq i64 %646, 0
  br i1 %654, label %.loopexit74, label %655

655:                                              ; preds = %636
  %656 = inttoptr i64 %639 to ptr
  %657 = select i1 %644, ptr %656, ptr %649
  %658 = getelementptr inbounds nuw i8, ptr %4, i64 632
  br label %661

659:                                              ; preds = %661
  %660 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<purrdf_core::small::IntoIter<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_core::small::IntoIter<[purrdf_sparql_eval::witness::RelationWitness; 1]>>(ptr noalias nofree noundef align 8 dereferenceable(48) %12) #89
          to label %669 unwind label %667, !noalias !23572

661:                                              ; preds = %665, %655
  %662 = phi i64 [ 0, %655 ], [ %663, %665 ]
  %663 = add nuw i64 %662, 1
  store i64 %663, ptr %652, align 8, !alias.scope !23573, !noalias !23576
  %664 = getelementptr inbounds nuw [24 x i8], ptr %657, i64 %662
  call void @llvm.lifetime.start.p0(ptr nonnull %11), !noalias !23567
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %11, ptr noundef nonnull align 8 dereferenceable(24) %664, i64 24, i1 false), !noalias !23572
; invoke <purrdf_sparql_eval::witness::RelationWitness>::merge
  invoke void @<purrdf_sparql_eval::witness::RelationWitness>::merge(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %658, ptr noalias nofree noundef nonnull readonly align 8 captures(none) dereferenceable(24) %11)
          to label %665 unwind label %659, !noalias !23572

.loopexit74:                                      ; preds = %665, %636
; invoke core::ptr::drop_glue::<purrdf_core::small::IntoIter<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_core::small::IntoIter<[purrdf_sparql_eval::witness::RelationWitness; 1]>>(ptr noalias nofree noundef align 8 dereferenceable(48) %12)
          to label %673 unwind label %671

665:                                              ; preds = %661
  call void @llvm.lifetime.end.p0(ptr nonnull %11), !noalias !23567
  %666 = icmp eq i64 %663, %646
  br i1 %666, label %.loopexit74, label %661

667:                                              ; preds = %659
  %668 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #90, !noalias !23572
  unreachable

669:                                              ; preds = %707, %671, %659
  %670 = phi { ptr, i32 } [ %660, %659 ], [ %672, %671 ], [ %708, %707 ]
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %32) #89, !noalias !23478
  br label %564

671:                                              ; preds = %.loopexit73, %751, %686, %684, %.loopexit74
  %672 = landingpad { ptr, i32 }
          cleanup
  br label %669

673:                                              ; preds = %.loopexit74
  call void @llvm.lifetime.end.p0(ptr nonnull %12), !noalias !23567
  %674 = trunc nuw i64 %628 to i1
  br i1 %674, label %675, label %677

675:                                              ; preds = %673
  %676 = icmp ugt i64 %627, %181
  br i1 %676, label %686, label %684, !prof !1735

677:                                              ; preds = %928, %673
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %42, ptr noundef nonnull align 8 dereferenceable(24) %32, i64 24, i1 false), !noalias !23472
  call void @llvm.lifetime.end.p0(ptr nonnull %32), !noalias !23472
  call void @llvm.lifetime.end.p0(ptr nonnull %35), !noalias !23472
  call void @llvm.lifetime.end.p0(ptr nonnull %36), !noalias !23472
  call void @llvm.experimental.noalias.scope.decl(metadata !23578)
  %678 = load ptr, ptr %41, align 8, !alias.scope !23578, !noalias !23478, !noundef !1733
  %679 = icmp eq ptr %678, null
  br i1 %679, label %930, label %680

680:                                              ; preds = %677
  %681 = atomicrmw sub ptr %678, i64 1 release, align 8, !noalias !23581
  %682 = icmp eq i64 %681, 1
  br i1 %682, label %683, label %930

683:                                              ; preds = %680
  fence acquire, !noalias !23478
; invoke <alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %41) #91
          to label %930 unwind label %526, !inline_history !2018

684:                                              ; preds = %675
  %685 = sub nuw nsw i64 %181, %627
  call void @llvm.lifetime.start.p0(ptr nonnull %28), !noalias !23472
; invoke <purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::for_rows::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::for_rows::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef align 8 captures(none) dereferenceable(200) %28, ptr noundef nonnull align 16 dereferenceable(1232) %4, i1 noundef zeroext false, i64 noundef %685)
          to label %687 unwind label %671

686:                                              ; preds = %675
; invoke core::slice::index::slice_index_fail
  invoke void @core::slice::index::slice_index_fail(i64 noundef %627, i64 noundef %181, i64 noundef %181, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.68dd637f94a7f528fe69f6876e3d956b.404) #93
          to label %929 unwind label %671, !noalias !23478

687:                                              ; preds = %684
  %688 = getelementptr inbounds nuw [40 x i8], ptr %550, i64 %181
  %689 = icmp samesign eq i64 %627, %181
  br i1 %689, label %.loopexit73, label %690

690:                                              ; preds = %687
  %691 = getelementptr inbounds nuw [40 x i8], ptr %550, i64 %627
  %692 = getelementptr inbounds nuw i8, ptr %10, i64 8
  %693 = getelementptr inbounds nuw i8, ptr %10, i64 16
  %694 = getelementptr inbounds nuw i8, ptr %10, i64 24
  %695 = getelementptr inbounds nuw i8, ptr %32, i64 16
  %696 = getelementptr inbounds nuw i8, ptr %32, i64 8
  %697 = getelementptr inbounds nuw i8, ptr %7, i64 8
  %698 = getelementptr inbounds nuw i8, ptr %7, i64 12
  %699 = getelementptr inbounds nuw i8, ptr %6, i64 8
  %wide.gep393 = getelementptr inbounds nuw [8 x i8], ptr %692, <4 x i64> <i64 0, i64 1, i64 2, i64 3>
  %wide.gep394 = getelementptr inbounds nuw i8, <4 x ptr> %wide.gep393, i64 4
  br label %700

700:                                              ; preds = %903, %690
  %701 = phi ptr [ %691, %690 ], [ %702, %903 ]
  %702 = getelementptr inbounds nuw i8, ptr %701, i64 40
  call void @llvm.lifetime.start.p0(ptr nonnull %27), !noalias !23472
; invoke <purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::pass::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::pass::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %27, ptr noalias nofree noundef align 8 dereferenceable(200) %28, ptr noundef nonnull align 16 dereferenceable(1232) %4)
          to label %709 unwind label %703, !noalias !23478

703:                                              ; preds = %740, %722, %700
  %704 = landingpad { ptr, i32 }
          cleanup
  br label %707

705:                                              ; preds = %794
  %706 = landingpad { ptr, i32 }
          cleanup
  br label %707

707:                                              ; preds = %916, %913, %705, %703
  %708 = phi { ptr, i32 } [ %914, %913 ], [ %914, %916 ], [ %704, %703 ], [ %706, %705 ]
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(200) %28)
          to label %669 unwind label %543

709:                                              ; preds = %700
  %710 = load i8, ptr %27, align 8, !range !1736, !noalias !23472, !noundef !1733
  %711 = icmp eq i8 %710, -1
  call void @llvm.lifetime.end.p0(ptr nonnull %27), !noalias !23472
  br i1 %711, label %712, label %.loopexit73

712:                                              ; preds = %709
  call void @llvm.lifetime.start.p0(ptr nonnull %26)
  %713 = load i64, ptr %701, align 8, !range !1771, !noalias !23478, !noundef !1733
  %714 = add i64 %713, -1
  %715 = icmp ugt i64 %714, 4
  %716 = getelementptr inbounds nuw i8, ptr %701, i64 8
  br i1 %715, label %717, label %722

717:                                              ; preds = %712
  %718 = load ptr, ptr %716, align 8, !noalias !23478, !nonnull !1733, !noundef !1733
  %719 = getelementptr inbounds nuw i8, ptr %701, i64 16
  %720 = load i64, ptr %719, align 8, !noalias !23478, !noundef !1733
  %721 = add i64 %720, -1
  br label %722

722:                                              ; preds = %717, %712
  %723 = phi i64 [ %721, %717 ], [ %714, %712 ]
  %724 = phi ptr [ %718, %717 ], [ %716, %712 ]
  %725 = load ptr, ptr %45, align 8, !noalias !23472, !nonnull !1733, !noundef !1733
  %726 = getelementptr inbounds nuw i8, ptr %725, i64 16
  call void @llvm.lifetime.start.p0(ptr nonnull %7), !noalias !23584
; invoke <purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::term::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::term::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef align 16 captures(none) dereferenceable(96) %7, ptr noalias nofree noundef nonnull align 8 dereferenceable(216) %43, ptr noalias nofree noundef nonnull readonly align 4 captures(address, read_provenance) %724, i64 noundef range(i64 0, 1152921504606846976) %723, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(56) %726, ptr noalias nofree noundef nonnull align 16 dereferenceable(1232) %4)
          to label %727 unwind label %703, !inline_history !23509

727:                                              ; preds = %722
  %728 = load i64, ptr %7, align 16, !range !2520, !noalias !23584, !noundef !1733
  %729 = icmp eq i64 %728, -1
  %730 = load i32, ptr %697, align 8, !noalias !23584
  %731 = load i32, ptr %698, align 4, !noalias !23584
  br i1 %729, label %737, label %732

732:                                              ; preds = %727
  %733 = getelementptr inbounds nuw i8, ptr %7, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(80) %26, ptr noundef nonnull align 16 dereferenceable(80) %733, i64 80, i1 false), !noalias !23591
  call void @llvm.lifetime.end.p0(ptr nonnull %7), !noalias !23584
  %734 = trunc i32 %730 to i8
  %735 = lshr i32 %730, 8
  %736 = trunc nuw i32 %735 to i24
  br label %751

737:                                              ; preds = %727
  call void @llvm.lifetime.end.p0(ptr nonnull %7), !noalias !23584
  %738 = icmp eq i32 %730, 2
  br i1 %738, label %739, label %740

739:                                              ; preds = %737
  call void @llvm.lifetime.end.p0(ptr nonnull %26)
  br label %903

740:                                              ; preds = %737
  call void @llvm.lifetime.start.p0(ptr nonnull %6), !noalias !23584
; invoke purrdf_sparql_eval::expr::ebv_term::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @purrdf_sparql_eval::expr::ebv_term::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef align 16 captures(none) dereferenceable(96) %6, ptr noalias nofree noundef nonnull align 16 dereferenceable(1232) %4, i32 noundef %730, i32 noundef %731)
          to label %741 unwind label %703, !inline_history !23509

741:                                              ; preds = %740
  %742 = load i64, ptr %6, align 16, !range !2520, !noalias !23584, !noundef !1733
  %743 = icmp eq i64 %742, -1
  %744 = load i8, ptr %699, align 8, !noalias !23584
  br i1 %743, label %761, label %745

745:                                              ; preds = %741
  %746 = getelementptr inbounds nuw i8, ptr %6, i64 9
  %747 = load i24, ptr %746, align 1, !noalias !23591
  %748 = getelementptr inbounds nuw i8, ptr %6, i64 12
  %749 = load i32, ptr %748, align 4, !noalias !23591
  %750 = getelementptr inbounds nuw i8, ptr %6, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(80) %26, ptr noundef nonnull align 16 dereferenceable(80) %750, i64 80, i1 false), !noalias !23591
  call void @llvm.lifetime.end.p0(ptr nonnull %6), !noalias !23584
  br label %751

751:                                              ; preds = %745, %732
  %752 = phi i24 [ %736, %732 ], [ %747, %745 ]
  %753 = phi i8 [ %734, %732 ], [ %744, %745 ]
  %754 = phi i32 [ %731, %732 ], [ %749, %745 ]
  %755 = phi i64 [ %728, %732 ], [ %742, %745 ]
  %756 = getelementptr inbounds nuw i8, ptr %0, i64 25
  store i24 %752, ptr %756, align 1, !noalias !23514
  %757 = getelementptr inbounds nuw i8, ptr %0, i64 28
  store i32 %754, ptr %757, align 4, !noalias !23514
  %758 = getelementptr inbounds nuw i8, ptr %0, i64 32
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(80) %758, ptr noundef nonnull align 16 dereferenceable(80) %26, i64 80, i1 false), !noalias !23514
  call void @llvm.lifetime.end.p0(ptr nonnull %26)
  %759 = getelementptr inbounds nuw i8, ptr %0, i64 16
  store i64 %755, ptr %759, align 16, !alias.scope !23467, !noalias !23514
  %760 = getelementptr inbounds nuw i8, ptr %0, i64 24
  store i8 %753, ptr %760, align 8, !alias.scope !23467, !noalias !23514
  store i64 1, ptr %0, align 16, !alias.scope !23467, !noalias !23514
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(200) %28)
          to label %926 unwind label %671

761:                                              ; preds = %741
  call void @llvm.lifetime.end.p0(ptr nonnull %6), !noalias !23584
  call void @llvm.lifetime.end.p0(ptr nonnull %26)
  %762 = and i8 %744, 1
  %763 = icmp eq i8 %762, 0
  br i1 %763, label %903, label %764

764:                                              ; preds = %761
  call void @llvm.lifetime.start.p0(ptr nonnull %25)
  call void @llvm.experimental.noalias.scope.decl(metadata !23592)
  %765 = load i64, ptr %701, align 8, !range !1771, !alias.scope !23592, !noalias !23595, !noundef !1733
  %766 = add i64 %765, -1
  %767 = icmp ugt i64 %766, 4
  %768 = getelementptr inbounds nuw i8, ptr %701, i64 16
  %769 = load i64, ptr %768, align 8, !alias.scope !23592, !noalias !23595
  %770 = add i64 %769, -1
  %771 = select i1 %767, i64 %770, i64 %766
  %772 = icmp ugt i64 %771, 4
  br i1 %772, label %782, label %773

773:                                              ; preds = %764
  call void @llvm.lifetime.start.p0(ptr nonnull %10), !noalias !23597
  %774 = icmp ugt i64 %765, 5
  %775 = load ptr, ptr %716, align 8, !alias.scope !23592, !noalias !23595, !nonnull !1733
  %776 = select i1 %774, ptr %775, ptr %716
  %777 = icmp eq i64 %771, 0
  br i1 %777, label %790, label %vector.body389

vector.body389:                                   ; preds = %773
  %trip.count.minus.1 = add nsw i64 %771, -1
  %broadcast.splatinsert = insertelement <4 x i64> poison, i64 %trip.count.minus.1, i64 0
  %broadcast.splat = shufflevector <4 x i64> %broadcast.splatinsert, <4 x i64> poison, <4 x i32> zeroinitializer
  %778 = icmp uge <4 x i64> %broadcast.splat, <i64 0, i64 1, i64 2, i64 3>
  %wide.gep = getelementptr inbounds nuw [8 x i8], ptr %776, <4 x i64> <i64 0, i64 1, i64 2, i64 3>
  %wide.masked.gather = call <4 x i32> @llvm.masked.gather.v4i32.v4p0(<4 x ptr> align 4 %wide.gep, <4 x i1> %778, <4 x i32> poison), !noalias !23595
  %wide.gep391 = getelementptr i8, <4 x ptr> %wide.gep, i64 4
  %wide.masked.gather392 = call <4 x i32> @llvm.masked.gather.v4i32.v4p0(<4 x ptr> align 4 %wide.gep391, <4 x i1> %778, <4 x i32> poison), !noalias !23595
  %779 = icmp eq <4 x i32> %wide.masked.gather, splat (i32 2)
  %780 = select <4 x i1> %779, <4 x i32> undef, <4 x i32> %wide.masked.gather392
  call void @llvm.masked.scatter.v4i32.v4p0(<4 x i32> %wide.masked.gather, <4 x ptr> align 4 %wide.gep393, <4 x i1> %778), !noalias !23597
  call void @llvm.masked.scatter.v4i32.v4p0(<4 x i32> %780, <4 x ptr> align 4 %wide.gep394, <4 x i1> %778), !noalias !23597
  %781 = add nuw nsw i64 %771, 1
  br label %790

782:                                              ; preds = %764
  %783 = shl i64 %771, 3
  %784 = icmp ugt i64 %771, 2305843009213693951
  %785 = icmp ugt i64 %783, 9223372036854775804
  %786 = or i1 %784, %785
  br i1 %786, label %794, label %787, !prof !6969

787:                                              ; preds = %782
; call __rustc::__rust_alloc
  %788 = call noundef align 4 ptr @__rustc::__rust_alloc(i64 noundef %783, i64 noundef range(i64 1, -9223372036854775807) 4) #92, !noalias !23598
  %789 = icmp eq ptr %788, null
  br i1 %789, label %794, label %iter.check

790:                                              ; preds = %vector.body389, %773
  %791 = phi i64 [ 1, %773 ], [ %781, %vector.body389 ]
  store i64 %791, ptr %10, align 8, !noalias !23597
  %792 = load ptr, ptr %692, align 8, !noalias !23601
  %793 = load i64, ptr %693, align 8, !noalias !23601
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %25, ptr noundef nonnull align 8 dereferenceable(16) %694, i64 16, i1 false), !noalias !23601
  call void @llvm.lifetime.end.p0(ptr nonnull %10), !noalias !23597
  br label %905

794:                                              ; preds = %787, %782
  %795 = phi i64 [ 4, %787 ], [ 0, %782 ]
; invoke alloc::raw_vec::handle_error
  invoke void @alloc::raw_vec::handle_error(i64 noundef %795, i64 %783) #93
          to label %796 unwind label %705

796:                                              ; preds = %794
  unreachable

iter.check:                                       ; preds = %787
  %797 = load ptr, ptr %716, align 8, !alias.scope !23592, !noalias !23595, !nonnull !1733
  %798 = select i1 %767, ptr %797, ptr %716
  %min.iters.check = icmp ult i64 %771, 8
  br i1 %min.iters.check, label %vec.epilog.scalar.ph.preheader, label %vector.memcheck

vector.memcheck:                                  ; preds = %iter.check
  %scevgep = getelementptr i8, ptr %788, i64 %783
  %scevgep369 = getelementptr i8, ptr %798, i64 %783
  %bound0 = icmp ult ptr %788, %scevgep369
  %bound1 = icmp ult ptr %798, %scevgep
  %found.conflict = and i1 %bound0, %bound1
  br i1 %found.conflict, label %vec.epilog.scalar.ph.preheader, label %vector.main.loop.iter.check

vector.main.loop.iter.check:                      ; preds = %vector.memcheck
  %min.iters.check370 = icmp ult i64 %771, 32
  br i1 %min.iters.check370, label %vec.epilog.ph, label %vector.ph

vector.ph:                                        ; preds = %vector.main.loop.iter.check
  %n.mod.vf = and i64 %771, 24
  %n.vec = and i64 %771, 2305843009213693920
  br label %vector.body

vector.body:                                      ; preds = %vector.body, %vector.ph
  %index = phi i64 [ 0, %vector.ph ], [ %index.next, %vector.body ]
  %799 = or disjoint i64 %index, 16
  %800 = getelementptr inbounds nuw [8 x i8], ptr %798, i64 %index
  %801 = getelementptr inbounds nuw [8 x i8], ptr %798, i64 %799
  %wide.vec = load <32 x i32>, ptr %800, align 4, !alias.scope !23602, !noalias !23605
  %strided.vec = shufflevector <32 x i32> %wide.vec, <32 x i32> poison, <16 x i32> <i32 0, i32 2, i32 4, i32 6, i32 8, i32 10, i32 12, i32 14, i32 16, i32 18, i32 20, i32 22, i32 24, i32 26, i32 28, i32 30>
  %strided.vec371 = shufflevector <32 x i32> %wide.vec, <32 x i32> poison, <16 x i32> <i32 1, i32 3, i32 5, i32 7, i32 9, i32 11, i32 13, i32 15, i32 17, i32 19, i32 21, i32 23, i32 25, i32 27, i32 29, i32 31>
  %wide.vec372 = load <32 x i32>, ptr %801, align 4, !alias.scope !23602, !noalias !23605
  %strided.vec373 = shufflevector <32 x i32> %wide.vec372, <32 x i32> poison, <16 x i32> <i32 0, i32 2, i32 4, i32 6, i32 8, i32 10, i32 12, i32 14, i32 16, i32 18, i32 20, i32 22, i32 24, i32 26, i32 28, i32 30>
  %strided.vec374 = shufflevector <32 x i32> %wide.vec372, <32 x i32> poison, <16 x i32> <i32 1, i32 3, i32 5, i32 7, i32 9, i32 11, i32 13, i32 15, i32 17, i32 19, i32 21, i32 23, i32 25, i32 27, i32 29, i32 31>
  %802 = icmp eq <16 x i32> %strided.vec, splat (i32 2)
  %803 = icmp eq <16 x i32> %strided.vec373, splat (i32 2)
  %804 = select <16 x i1> %802, <16 x i32> undef, <16 x i32> %strided.vec371
  %805 = select <16 x i1> %803, <16 x i32> undef, <16 x i32> %strided.vec374
  %806 = getelementptr inbounds nuw [8 x i8], ptr %788, i64 %index
  %807 = getelementptr inbounds nuw [8 x i8], ptr %788, i64 %799
  %interleaved.vec = shufflevector <16 x i32> %strided.vec, <16 x i32> %804, <32 x i32> <i32 0, i32 16, i32 1, i32 17, i32 2, i32 18, i32 3, i32 19, i32 4, i32 20, i32 5, i32 21, i32 6, i32 22, i32 7, i32 23, i32 8, i32 24, i32 9, i32 25, i32 10, i32 26, i32 11, i32 27, i32 12, i32 28, i32 13, i32 29, i32 14, i32 30, i32 15, i32 31>
  store <32 x i32> %interleaved.vec, ptr %806, align 4, !alias.scope !23618, !noalias !23620
  %interleaved.vec375 = shufflevector <16 x i32> %strided.vec373, <16 x i32> %805, <32 x i32> <i32 0, i32 16, i32 1, i32 17, i32 2, i32 18, i32 3, i32 19, i32 4, i32 20, i32 5, i32 21, i32 6, i32 22, i32 7, i32 23, i32 8, i32 24, i32 9, i32 25, i32 10, i32 26, i32 11, i32 27, i32 12, i32 28, i32 13, i32 29, i32 14, i32 30, i32 15, i32 31>
  store <32 x i32> %interleaved.vec375, ptr %807, align 4, !alias.scope !23618, !noalias !23620
  %index.next = add nuw i64 %index, 32
  %808 = icmp eq i64 %index.next, %n.vec
  br i1 %808, label %middle.block, label %vector.body, !llvm.loop !23627

middle.block:                                     ; preds = %vector.body
  %ind.escape = add nsw i64 %n.vec, -1
  %cmp.n = icmp eq i64 %771, %n.vec
  br i1 %cmp.n, label %.loopexit397, label %vec.epilog.iter.check

vec.epilog.iter.check:                            ; preds = %middle.block
  %min.epilog.iters.check = icmp eq i64 %n.mod.vf, 0
  br i1 %min.epilog.iters.check, label %vec.epilog.scalar.ph.preheader, label %vec.epilog.ph, !prof !6205

vec.epilog.ph:                                    ; preds = %vector.main.loop.iter.check, %vec.epilog.iter.check
  %vec.epilog.resume.val = phi i64 [ %n.vec, %vec.epilog.iter.check ], [ 0, %vector.main.loop.iter.check ]
  %n.vec377 = and i64 %771, 2305843009213693944
  br label %vec.epilog.vector.body

vec.epilog.vector.body:                           ; preds = %vec.epilog.vector.body, %vec.epilog.ph
  %index378 = phi i64 [ %vec.epilog.resume.val, %vec.epilog.ph ], [ %index.next383, %vec.epilog.vector.body ]
  %809 = getelementptr inbounds nuw [8 x i8], ptr %798, i64 %index378
  %wide.vec379 = load <16 x i32>, ptr %809, align 4, !alias.scope !23602, !noalias !23605
  %strided.vec380 = shufflevector <16 x i32> %wide.vec379, <16 x i32> poison, <8 x i32> <i32 0, i32 2, i32 4, i32 6, i32 8, i32 10, i32 12, i32 14>
  %strided.vec381 = shufflevector <16 x i32> %wide.vec379, <16 x i32> poison, <8 x i32> <i32 1, i32 3, i32 5, i32 7, i32 9, i32 11, i32 13, i32 15>
  %810 = icmp eq <8 x i32> %strided.vec380, splat (i32 2)
  %811 = select <8 x i1> %810, <8 x i32> undef, <8 x i32> %strided.vec381
  %812 = getelementptr inbounds nuw [8 x i8], ptr %788, i64 %index378
  %interleaved.vec382 = shufflevector <8 x i32> %strided.vec380, <8 x i32> %811, <16 x i32> <i32 0, i32 8, i32 1, i32 9, i32 2, i32 10, i32 3, i32 11, i32 4, i32 12, i32 5, i32 13, i32 6, i32 14, i32 7, i32 15>
  store <16 x i32> %interleaved.vec382, ptr %812, align 4, !alias.scope !23618, !noalias !23620
  %index.next383 = add nuw i64 %index378, 8
  %813 = icmp eq i64 %index.next383, %n.vec377
  br i1 %813, label %vec.epilog.middle.block, label %vec.epilog.vector.body, !llvm.loop !23628

vec.epilog.middle.block:                          ; preds = %vec.epilog.vector.body
  %ind.escape384 = add nsw i64 %n.vec377, -1
  %cmp.n385 = icmp eq i64 %771, %n.vec377
  br i1 %cmp.n385, label %.loopexit397, label %vec.epilog.scalar.ph.preheader

vec.epilog.scalar.ph.preheader:                   ; preds = %vector.memcheck, %iter.check, %vec.epilog.iter.check, %vec.epilog.middle.block
  %.ph = phi i64 [ 0, %iter.check ], [ 0, %vector.memcheck ], [ %n.vec, %vec.epilog.iter.check ], [ %n.vec377, %vec.epilog.middle.block ]
  %xtraiter = and i64 %771, 7
  %lcmp.mod.not = icmp eq i64 %xtraiter, 0
  br i1 %lcmp.mod.not, label %vec.epilog.scalar.ph.prol.loopexit, label %vec.epilog.scalar.ph.prol

vec.epilog.scalar.ph.prol:                        ; preds = %vec.epilog.scalar.ph.preheader, %vec.epilog.scalar.ph.prol
  %814 = phi i64 [ %823, %vec.epilog.scalar.ph.prol ], [ %.ph, %vec.epilog.scalar.ph.preheader ]
  %prol.iter = phi i64 [ %prol.iter.next, %vec.epilog.scalar.ph.prol ], [ 0, %vec.epilog.scalar.ph.preheader ]
  %815 = getelementptr inbounds nuw [8 x i8], ptr %798, i64 %814
  %816 = load i32, ptr %815, align 4, !range !1778, !noalias !23605, !noundef !1733
  %817 = getelementptr i8, ptr %815, i64 4
  %818 = load i32, ptr %817, align 4, !noalias !23605
  %819 = icmp eq i32 %816, 2
  %820 = select i1 %819, i32 undef, i32 %818
  %821 = getelementptr inbounds nuw [8 x i8], ptr %788, i64 %814
  store i32 %816, ptr %821, align 4, !noalias !23620
  %822 = getelementptr inbounds nuw i8, ptr %821, i64 4
  store i32 %820, ptr %822, align 4, !noalias !23620
  %823 = add nuw nsw i64 %814, 1
  %prol.iter.next = add i64 %prol.iter, 1
  %prol.iter.cmp.not = icmp eq i64 %prol.iter.next, %xtraiter
  br i1 %prol.iter.cmp.not, label %vec.epilog.scalar.ph.prol.loopexit, label %vec.epilog.scalar.ph.prol, !llvm.loop !23629

vec.epilog.scalar.ph.prol.loopexit:               ; preds = %vec.epilog.scalar.ph.prol, %vec.epilog.scalar.ph.preheader
  %.lcssa408.unr = phi i64 [ poison, %vec.epilog.scalar.ph.preheader ], [ %814, %vec.epilog.scalar.ph.prol ]
  %.unr = phi i64 [ %.ph, %vec.epilog.scalar.ph.preheader ], [ %823, %vec.epilog.scalar.ph.prol ]
  %824 = sub nsw i64 %.ph, %771
  %825 = icmp ugt i64 %824, -8
  br i1 %825, label %.loopexit397, label %vec.epilog.scalar.ph

vec.epilog.scalar.ph:                             ; preds = %vec.epilog.scalar.ph.prol.loopexit, %vec.epilog.scalar.ph
  %826 = phi i64 [ %898, %vec.epilog.scalar.ph ], [ %.unr, %vec.epilog.scalar.ph.prol.loopexit ]
  %827 = getelementptr inbounds nuw [8 x i8], ptr %798, i64 %826
  %828 = load i32, ptr %827, align 4, !range !1778, !noalias !23605, !noundef !1733
  %829 = getelementptr i8, ptr %827, i64 4
  %830 = load i32, ptr %829, align 4, !noalias !23605
  %831 = icmp eq i32 %828, 2
  %832 = select i1 %831, i32 undef, i32 %830
  %833 = getelementptr inbounds nuw [8 x i8], ptr %788, i64 %826
  store i32 %828, ptr %833, align 4, !noalias !23620
  %834 = getelementptr inbounds nuw i8, ptr %833, i64 4
  store i32 %832, ptr %834, align 4, !noalias !23620
  %835 = add nuw nsw i64 %826, 1
  %836 = getelementptr inbounds nuw [8 x i8], ptr %798, i64 %835
  %837 = load i32, ptr %836, align 4, !range !1778, !noalias !23605, !noundef !1733
  %838 = getelementptr i8, ptr %836, i64 4
  %839 = load i32, ptr %838, align 4, !noalias !23605
  %840 = icmp eq i32 %837, 2
  %841 = select i1 %840, i32 undef, i32 %839
  %842 = getelementptr inbounds nuw [8 x i8], ptr %788, i64 %835
  store i32 %837, ptr %842, align 4, !noalias !23620
  %843 = getelementptr inbounds nuw i8, ptr %842, i64 4
  store i32 %841, ptr %843, align 4, !noalias !23620
  %844 = add nuw nsw i64 %826, 2
  %845 = getelementptr inbounds nuw [8 x i8], ptr %798, i64 %844
  %846 = load i32, ptr %845, align 4, !range !1778, !noalias !23605, !noundef !1733
  %847 = getelementptr i8, ptr %845, i64 4
  %848 = load i32, ptr %847, align 4, !noalias !23605
  %849 = icmp eq i32 %846, 2
  %850 = select i1 %849, i32 undef, i32 %848
  %851 = getelementptr inbounds nuw [8 x i8], ptr %788, i64 %844
  store i32 %846, ptr %851, align 4, !noalias !23620
  %852 = getelementptr inbounds nuw i8, ptr %851, i64 4
  store i32 %850, ptr %852, align 4, !noalias !23620
  %853 = add nuw nsw i64 %826, 3
  %854 = getelementptr inbounds nuw [8 x i8], ptr %798, i64 %853
  %855 = load i32, ptr %854, align 4, !range !1778, !noalias !23605, !noundef !1733
  %856 = getelementptr i8, ptr %854, i64 4
  %857 = load i32, ptr %856, align 4, !noalias !23605
  %858 = icmp eq i32 %855, 2
  %859 = select i1 %858, i32 undef, i32 %857
  %860 = getelementptr inbounds nuw [8 x i8], ptr %788, i64 %853
  store i32 %855, ptr %860, align 4, !noalias !23620
  %861 = getelementptr inbounds nuw i8, ptr %860, i64 4
  store i32 %859, ptr %861, align 4, !noalias !23620
  %862 = add nuw nsw i64 %826, 4
  %863 = getelementptr inbounds nuw [8 x i8], ptr %798, i64 %862
  %864 = load i32, ptr %863, align 4, !range !1778, !noalias !23605, !noundef !1733
  %865 = getelementptr i8, ptr %863, i64 4
  %866 = load i32, ptr %865, align 4, !noalias !23605
  %867 = icmp eq i32 %864, 2
  %868 = select i1 %867, i32 undef, i32 %866
  %869 = getelementptr inbounds nuw [8 x i8], ptr %788, i64 %862
  store i32 %864, ptr %869, align 4, !noalias !23620
  %870 = getelementptr inbounds nuw i8, ptr %869, i64 4
  store i32 %868, ptr %870, align 4, !noalias !23620
  %871 = add nuw nsw i64 %826, 5
  %872 = getelementptr inbounds nuw [8 x i8], ptr %798, i64 %871
  %873 = load i32, ptr %872, align 4, !range !1778, !noalias !23605, !noundef !1733
  %874 = getelementptr i8, ptr %872, i64 4
  %875 = load i32, ptr %874, align 4, !noalias !23605
  %876 = icmp eq i32 %873, 2
  %877 = select i1 %876, i32 undef, i32 %875
  %878 = getelementptr inbounds nuw [8 x i8], ptr %788, i64 %871
  store i32 %873, ptr %878, align 4, !noalias !23620
  %879 = getelementptr inbounds nuw i8, ptr %878, i64 4
  store i32 %877, ptr %879, align 4, !noalias !23620
  %880 = add nuw nsw i64 %826, 6
  %881 = getelementptr inbounds nuw [8 x i8], ptr %798, i64 %880
  %882 = load i32, ptr %881, align 4, !range !1778, !noalias !23605, !noundef !1733
  %883 = getelementptr i8, ptr %881, i64 4
  %884 = load i32, ptr %883, align 4, !noalias !23605
  %885 = icmp eq i32 %882, 2
  %886 = select i1 %885, i32 undef, i32 %884
  %887 = getelementptr inbounds nuw [8 x i8], ptr %788, i64 %880
  store i32 %882, ptr %887, align 4, !noalias !23620
  %888 = getelementptr inbounds nuw i8, ptr %887, i64 4
  store i32 %886, ptr %888, align 4, !noalias !23620
  %889 = add nuw nsw i64 %826, 7
  %890 = getelementptr inbounds nuw [8 x i8], ptr %798, i64 %889
  %891 = load i32, ptr %890, align 4, !range !1778, !noalias !23605, !noundef !1733
  %892 = getelementptr i8, ptr %890, i64 4
  %893 = load i32, ptr %892, align 4, !noalias !23605
  %894 = icmp eq i32 %891, 2
  %895 = select i1 %894, i32 undef, i32 %893
  %896 = getelementptr inbounds nuw [8 x i8], ptr %788, i64 %889
  store i32 %891, ptr %896, align 4, !noalias !23620
  %897 = getelementptr inbounds nuw i8, ptr %896, i64 4
  store i32 %895, ptr %897, align 4, !noalias !23620
  %898 = add nuw nsw i64 %826, 8
  %899 = icmp eq i64 %898, %771
  br i1 %899, label %.loopexit397, label %vec.epilog.scalar.ph, !llvm.loop !23630

.loopexit397:                                     ; preds = %vec.epilog.scalar.ph.prol.loopexit, %vec.epilog.scalar.ph, %vec.epilog.middle.block, %middle.block
  %.lcssa = phi i64 [ %ind.escape384, %vec.epilog.middle.block ], [ %ind.escape, %middle.block ], [ %.lcssa408.unr, %vec.epilog.scalar.ph.prol.loopexit ], [ %889, %vec.epilog.scalar.ph ]
  %900 = icmp samesign ult i64 %771, 1152921504606846976
  call void @llvm.assume(i1 %900), !noalias !23478
  %901 = add nuw nsw i64 %.lcssa, 2
  %902 = add nuw nsw i64 %771, 1
  br label %905

903:                                              ; preds = %919, %761, %739
  %904 = icmp eq ptr %702, %688
  br i1 %904, label %.loopexit73, label %700

905:                                              ; preds = %.loopexit397, %790
  %906 = phi ptr [ %788, %.loopexit397 ], [ %792, %790 ]
  %907 = phi i64 [ %902, %.loopexit397 ], [ %791, %790 ]
  %908 = phi i64 [ %901, %.loopexit397 ], [ %793, %790 ]
  call void @llvm.experimental.noalias.scope.decl(metadata !23631)
  %909 = load i64, ptr %695, align 8, !alias.scope !23631, !noalias !23634, !noundef !1733
  %910 = load i64, ptr %32, align 8, !range !1828, !alias.scope !23631, !noalias !23634, !noundef !1733
  %911 = icmp eq i64 %909, %910
  br i1 %911, label %912, label %919

912:                                              ; preds = %905
; invoke <alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one
  invoke void @<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %32)
          to label %919 unwind label %913, !noalias !23634

913:                                              ; preds = %912
  %914 = landingpad { ptr, i32 }
          cleanup
  %915 = icmp samesign ugt i64 %907, 5
  br i1 %915, label %916, label %707

916:                                              ; preds = %913
  %917 = shl nuw i64 %907, 3
  %918 = add i64 %917, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %906, i64 noundef %918, i64 noundef range(i64 1, -9223372036854775807) 4) #92, !noalias !23636
  br label %707

919:                                              ; preds = %912, %905
  %920 = load ptr, ptr %696, align 8, !alias.scope !23631, !noalias !23634, !nonnull !1733, !noundef !1733
  %921 = getelementptr inbounds nuw [40 x i8], ptr %920, i64 %909
  store i64 %907, ptr %921, align 8, !noalias !23639
  %922 = getelementptr inbounds nuw i8, ptr %921, i64 8
  store ptr %906, ptr %922, align 8, !noalias !23639
  %923 = getelementptr inbounds nuw i8, ptr %921, i64 16
  store i64 %908, ptr %923, align 8, !noalias !23639
  %924 = getelementptr inbounds nuw i8, ptr %921, i64 24
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %924, ptr noundef nonnull align 8 dereferenceable(16) %25, i64 16, i1 false), !noalias !23639
  %925 = add i64 %909, 1
  store i64 %925, ptr %695, align 8, !alias.scope !23631, !noalias !23634
  call void @llvm.lifetime.end.p0(ptr nonnull %25)
  br label %903

926:                                              ; preds = %751
  call void @llvm.lifetime.end.p0(ptr nonnull %28), !noalias !23472
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %32), !noalias !23478
  call void @llvm.lifetime.end.p0(ptr nonnull %32), !noalias !23472
  call void @llvm.lifetime.end.p0(ptr nonnull %35), !noalias !23472
  br label %927

927:                                              ; preds = %926, %629
  call void @llvm.lifetime.end.p0(ptr nonnull %36), !noalias !23472
  br label %1147

.loopexit73:                                      ; preds = %903, %709, %687
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(200) %28)
          to label %928 unwind label %671

928:                                              ; preds = %.loopexit73
  call void @llvm.lifetime.end.p0(ptr nonnull %28), !noalias !23472
  br label %677

929:                                              ; preds = %686
  unreachable

930:                                              ; preds = %683, %680, %677
  call void @llvm.lifetime.end.p0(ptr nonnull %41), !noalias !23472
  br label %531

931:                                              ; preds = %996
  %932 = landingpad { ptr, i32 }
          cleanup
  br label %522

933:                                              ; preds = %531
  %934 = getelementptr inbounds nuw i8, ptr %534, i64 16
  %935 = load i8, ptr %934, align 8, !noalias !23640
  %936 = icmp eq i8 %935, -1
  br i1 %936, label %943, label %937

937:                                              ; preds = %933
  %938 = getelementptr inbounds nuw i8, ptr %534, i64 17
  call void @llvm.lifetime.start.p0(ptr nonnull %17), !noalias !23472
  store i8 %935, ptr %17, align 8, !noalias !23472
  %939 = getelementptr inbounds nuw i8, ptr %17, i64 1
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(23) %939, ptr noundef nonnull align 1 dereferenceable(23) %938, i64 23, i1 false)
  call void @llvm.lifetime.start.p0(ptr nonnull %16), !noalias !23472
  %940 = load ptr, ptr %45, align 8, !noalias !23472, !nonnull !1733, !noundef !1733
  %941 = atomicrmw add ptr %940, i64 1 monotonic, align 8, !noalias !23478
  %942 = icmp slt i64 %941, 0
  br i1 %942, label %999, label %997

943:                                              ; preds = %933, %531
  call void @llvm.lifetime.start.p0(ptr nonnull %15), !noalias !23472
  call void @llvm.lifetime.start.p0(ptr nonnull %14), !noalias !23472
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(104) %14, ptr noundef nonnull align 8 dereferenceable(104) %47, i64 104, i1 false), !noalias !23641
  call void @llvm.lifetime.start.p0(ptr nonnull %13), !noalias !23472
  %944 = load ptr, ptr %45, align 8, !noalias !23472, !nonnull !1733, !noundef !1733
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %13, ptr noundef nonnull align 8 dereferenceable(24) %42, i64 24, i1 false), !noalias !23472
  %945 = getelementptr inbounds nuw i8, ptr %13, i64 24
  store ptr %944, ptr %945, align 8, !noalias !23472
  call void @llvm.experimental.noalias.scope.decl(metadata !23642)
  call void @llvm.experimental.noalias.scope.decl(metadata !23645)
  call void @llvm.experimental.noalias.scope.decl(metadata !23647)
  %946 = load i64, ptr %14, align 8, !range !2052, !alias.scope !23645, !noalias !23649, !noundef !1733
  %947 = icmp eq i64 %946, -1
  br i1 %947, label %949, label %948

948:                                              ; preds = %943
; call <purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
  call fastcc void @<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(96) %15, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(32) %13, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(104) %47), !noalias !23650
  br label %951

949:                                              ; preds = %943
  %950 = getelementptr inbounds nuw i8, ptr %15, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %950, ptr noundef nonnull readonly align 8 dereferenceable(32) %13, i64 32, i1 false), !alias.scope !23651, !noalias !23652
  store i64 -1, ptr %15, align 8, !alias.scope !23642, !noalias !23653
  br label %951

951:                                              ; preds = %949, %948
  %952 = getelementptr inbounds nuw i8, ptr %14, i64 72
  %953 = load i64, ptr %952, align 8, !range !1771, !alias.scope !23654, !noalias !23649, !noundef !1733
  %954 = icmp ugt i64 %953, 5
  br i1 %954, label %955, label %989

955:                                              ; preds = %951
  %956 = getelementptr inbounds nuw i8, ptr %14, i64 80
  %957 = load ptr, ptr %956, align 8, !alias.scope !23645, !noalias !23649, !nonnull !1733, !noundef !1733
  %958 = mul i64 %953, 3
  %959 = add i64 %958, -3
  %960 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %961 = load i64, ptr %960, align 8, !noalias !23657, !noundef !1733
  %962 = call i64 @llvm.umin.i64(i64 %959, i64 9223372036854775807)
  %963 = call i64 @llvm.ssub.sat.i64(i64 %961, i64 %962)
  store i64 %963, ptr %960, align 8, !noalias !23657
  %964 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %965 = load i64, ptr %964, align 8, !noalias !23657, !noundef !1733
  %966 = icmp slt i64 %963, %965
  br i1 %966, label %967, label %.preheader405

967:                                              ; preds = %955
  store i64 %963, ptr %964, align 8, !noalias !23657
  br label %.preheader405

.preheader405:                                    ; preds = %967, %955
  br label %968

968:                                              ; preds = %.preheader405, %971
  %969 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !23657
  %970 = icmp slt i64 %969, 0
  br i1 %970, label %971, label %__rustc::__rust_dealloc (.exit60)

971:                                              ; preds = %968
  %972 = add nsw i64 %969, 1
  %973 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %969, i64 %972 acq_rel acquire, align 8, !noalias !23657
  %974 = extractvalue { i64, i1 } %973, 1
  br i1 %974, label %975, label %968

975:                                              ; preds = %971
  %976 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %962 monotonic, align 8, !noalias !23657
  %977 = call i64 @llvm.ssub.sat.i64(i64 %976, i64 %962)
  %978 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !23657
  br label %979

979:                                              ; preds = %982, %975
  %980 = phi i64 [ %978, %975 ], [ %985, %982 ]
  %981 = icmp slt i64 %977, %980
  br i1 %981, label %982, label %986

982:                                              ; preds = %979
  %983 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %980, i64 %977 monotonic monotonic, align 8, !noalias !23657
  %984 = extractvalue { i64, i1 } %983, 1
  %985 = extractvalue { i64, i1 } %983, 0
  br i1 %984, label %986, label %979

986:                                              ; preds = %982, %979
  %987 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !23657
  br label %__rustc::__rust_dealloc (.exit60)

__rustc::__rust_dealloc (.exit60): ; preds = %968, %986
  %988 = icmp ne i64 %959, 0
  call void @llvm.assume(i1 %988), !noalias !23657
  call void @free(ptr noundef nonnull %957) #92, !noalias !23657
  br label %989

989:                                              ; preds = %__rustc::__rust_dealloc (.exit60), %951
  %990 = getelementptr inbounds nuw i8, ptr %14, i64 96
  call void @llvm.experimental.noalias.scope.decl(metadata !23660), !noalias !23478
  %991 = load ptr, ptr %990, align 8, !alias.scope !23663, !noalias !23649, !noundef !1733
  %992 = icmp eq ptr %991, null
  br i1 %992, label %1083, label %993

993:                                              ; preds = %989
  %994 = atomicrmw sub ptr %991, i64 1 release, align 8, !noalias !23664
  %995 = icmp eq i64 %994, 1
  br i1 %995, label %996, label %1083

996:                                              ; preds = %993
  fence acquire, !noalias !23478
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %990) #91
          to label %1083 unwind label %931

997:                                              ; preds = %937
  %998 = load ptr, ptr %45, align 8, !noalias !23472, !nonnull !1733, !noundef !1733
; invoke <purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::barred_at
  invoke fastcc void @<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::barred_at(ptr noalias nofree noundef align 8 captures(address) dereferenceable(96) %16, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %1, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %17, ptr noundef nonnull %998)
          to label %1000 unwind label %1145, !noalias !23478

999:                                              ; preds = %937
  call void @llvm.trap()
  unreachable

1000:                                             ; preds = %997
  %1001 = getelementptr inbounds nuw i8, ptr %0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(96) %1001, ptr noundef nonnull align 8 dereferenceable(96) %16, i64 96, i1 false), !noalias !23514
  call void @llvm.lifetime.end.p0(ptr nonnull %16), !noalias !23472
  store i64 0, ptr %0, align 16, !alias.scope !23467, !noalias !23514
  call void @llvm.lifetime.end.p0(ptr nonnull %17), !noalias !23472
  call void @llvm.experimental.noalias.scope.decl(metadata !23669)
  %1002 = getelementptr inbounds nuw i8, ptr %42, i64 8
  %1003 = load ptr, ptr %1002, align 8, !alias.scope !23669, !noalias !23478, !nonnull !1733, !noundef !1733
  %1004 = getelementptr inbounds nuw i8, ptr %42, i64 16
  %1005 = load i64, ptr %1004, align 8, !alias.scope !23669, !noalias !23478, !noundef !1733
  call void @llvm.experimental.noalias.scope.decl(metadata !23672), !noalias !23478
  %1006 = icmp eq i64 %1005, 0
  br i1 %1006, label %.loopexit71, label %.preheader70

.preheader70:                                     ; preds = %1000
  %1007 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1008 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  br label %1009

1009:                                             ; preds = %.preheader70, %1047
  %1010 = phi i64 [ %1012, %1047 ], [ 0, %.preheader70 ]
  %1011 = getelementptr inbounds nuw [40 x i8], ptr %1003, i64 %1010
  %1012 = add nuw nsw i64 %1010, 1
  %1013 = load i64, ptr %1011, align 8, !range !1771, !alias.scope !23675, !noalias !23678, !noundef !1733
  %1014 = icmp ugt i64 %1013, 5
  br i1 %1014, label %1015, label %1047

1015:                                             ; preds = %1009
  %1016 = getelementptr i8, ptr %1011, i64 8
  %1017 = load ptr, ptr %1016, align 8, !alias.scope !23672, !noalias !23678, !nonnull !1733, !noundef !1733
  %1018 = shl i64 %1013, 3
  %1019 = add i64 %1018, -8
  %1020 = load i64, ptr %1007, align 8, !noalias !23679, !noundef !1733
  %1021 = call i64 @llvm.umin.i64(i64 %1019, i64 9223372036854775807)
  %1022 = call i64 @llvm.ssub.sat.i64(i64 %1020, i64 %1021)
  store i64 %1022, ptr %1007, align 8, !noalias !23679
  %1023 = load i64, ptr %1008, align 8, !noalias !23679, !noundef !1733
  %1024 = icmp slt i64 %1022, %1023
  br i1 %1024, label %1025, label %.preheader407

1025:                                             ; preds = %1015
  store i64 %1022, ptr %1008, align 8, !noalias !23679
  br label %.preheader407

.preheader407:                                    ; preds = %1025, %1015
  br label %1026

1026:                                             ; preds = %.preheader407, %1029
  %1027 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !23679
  %1028 = icmp slt i64 %1027, 0
  br i1 %1028, label %1029, label %__rustc::__rust_dealloc (.exit61)

1029:                                             ; preds = %1026
  %1030 = add nsw i64 %1027, 1
  %1031 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %1027, i64 %1030 acq_rel acquire, align 8, !noalias !23679
  %1032 = extractvalue { i64, i1 } %1031, 1
  br i1 %1032, label %1033, label %1026

1033:                                             ; preds = %1029
  %1034 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1021 monotonic, align 8, !noalias !23679
  %1035 = call i64 @llvm.ssub.sat.i64(i64 %1034, i64 %1021)
  %1036 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !23679
  br label %1037

1037:                                             ; preds = %1040, %1033
  %1038 = phi i64 [ %1036, %1033 ], [ %1043, %1040 ]
  %1039 = icmp slt i64 %1035, %1038
  br i1 %1039, label %1040, label %1044

1040:                                             ; preds = %1037
  %1041 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1038, i64 %1035 monotonic monotonic, align 8, !noalias !23679
  %1042 = extractvalue { i64, i1 } %1041, 1
  %1043 = extractvalue { i64, i1 } %1041, 0
  br i1 %1042, label %1044, label %1037

1044:                                             ; preds = %1040, %1037
  %1045 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !23679
  br label %__rustc::__rust_dealloc (.exit61)

__rustc::__rust_dealloc (.exit61): ; preds = %1026, %1044
  %1046 = icmp ne i64 %1019, 0
  call void @llvm.assume(i1 %1046), !noalias !23679
  call void @free(ptr noundef nonnull %1017) #92, !noalias !23679
  br label %1047

1047:                                             ; preds = %__rustc::__rust_dealloc (.exit61), %1009
  %1048 = icmp eq i64 %1012, %1005
  br i1 %1048, label %.loopexit71, label %1009

.loopexit71:                                      ; preds = %1047, %1000
  %1049 = load i64, ptr %42, align 8, !alias.scope !23669, !noalias !23478
  %1050 = icmp eq i64 %1049, 0
  br i1 %1050, label %1081, label %1051

1051:                                             ; preds = %.loopexit71
  %1052 = mul nuw i64 %1049, 40
  %1053 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1054 = load i64, ptr %1053, align 8, !noalias !23678, !noundef !1733
  %1055 = call i64 @llvm.umin.i64(i64 %1052, i64 9223372036854775807)
  %1056 = call i64 @llvm.ssub.sat.i64(i64 %1054, i64 %1055)
  store i64 %1056, ptr %1053, align 8, !noalias !23678
  %1057 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1058 = load i64, ptr %1057, align 8, !noalias !23678, !noundef !1733
  %1059 = icmp slt i64 %1056, %1058
  br i1 %1059, label %1060, label %.preheader406

1060:                                             ; preds = %1051
  store i64 %1056, ptr %1057, align 8, !noalias !23678
  br label %.preheader406

.preheader406:                                    ; preds = %1060, %1051
  br label %1061

1061:                                             ; preds = %.preheader406, %1064
  %1062 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !23678
  %1063 = icmp slt i64 %1062, 0
  br i1 %1063, label %1064, label %__rustc::__rust_dealloc (.exit62)

1064:                                             ; preds = %1061
  %1065 = add nsw i64 %1062, 1
  %1066 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %1062, i64 %1065 acq_rel acquire, align 8, !noalias !23678
  %1067 = extractvalue { i64, i1 } %1066, 1
  br i1 %1067, label %1068, label %1061

1068:                                             ; preds = %1064
  %1069 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1055 monotonic, align 8, !noalias !23678
  %1070 = call i64 @llvm.ssub.sat.i64(i64 %1069, i64 %1055)
  %1071 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !23678
  br label %1072

1072:                                             ; preds = %1075, %1068
  %1073 = phi i64 [ %1071, %1068 ], [ %1078, %1075 ]
  %1074 = icmp slt i64 %1070, %1073
  br i1 %1074, label %1075, label %1079

1075:                                             ; preds = %1072
  %1076 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1073, i64 %1070 monotonic monotonic, align 8, !noalias !23678
  %1077 = extractvalue { i64, i1 } %1076, 1
  %1078 = extractvalue { i64, i1 } %1076, 0
  br i1 %1077, label %1079, label %1072

1079:                                             ; preds = %1075, %1072
  %1080 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !23678
  br label %__rustc::__rust_dealloc (.exit62)

__rustc::__rust_dealloc (.exit62): ; preds = %1061, %1079
  call void @free(ptr noundef nonnull %1003) #92, !noalias !23678
  br label %1081

1081:                                             ; preds = %1158, %__rustc::__rust_dealloc (.exit62), %.loopexit71, %528
  %1082 = phi i8 [ 1, %1158 ], [ 0, %528 ], [ %532, %.loopexit71 ], [ %532, %__rustc::__rust_dealloc (.exit62) ]
  call void @llvm.lifetime.end.p0(ptr nonnull %42), !noalias !23472
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>(ptr noalias nofree noundef align 8 dereferenceable(216) %43)
          to label %1159 unwind label %189, !noalias !23478

1083:                                             ; preds = %996, %993, %989
  call void @llvm.lifetime.end.p0(ptr nonnull %13), !noalias !23472
  call void @llvm.lifetime.end.p0(ptr nonnull %14), !noalias !23472
  %1084 = getelementptr inbounds nuw i8, ptr %0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(96) %1084, ptr noundef nonnull align 8 dereferenceable(96) %15, i64 96, i1 false), !noalias !23514
  store i64 0, ptr %0, align 16, !alias.scope !23467, !noalias !23514
  call void @llvm.lifetime.end.p0(ptr nonnull %15), !noalias !23472
  call void @llvm.lifetime.end.p0(ptr nonnull %42), !noalias !23472
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>(ptr noalias nofree noundef align 8 dereferenceable(216) %43)
          to label %1085 unwind label %189, !noalias !23478

1085:                                             ; preds = %1083
  call void @llvm.lifetime.end.p0(ptr nonnull %43), !noalias !23472
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(200) %44)
          to label %1086 unwind label %155

1086:                                             ; preds = %1085
  call void @llvm.lifetime.end.p0(ptr nonnull %44), !noalias !23472
  call void @llvm.lifetime.end.p0(ptr nonnull %45), !noalias !23472
  call void @llvm.experimental.noalias.scope.decl(metadata !23682)
  call void @llvm.experimental.noalias.scope.decl(metadata !23685), !noalias !23688
  %1087 = load ptr, ptr %144, align 8, !alias.scope !23689, !noalias !23688, !nonnull !1733, !noundef !1733
  %1088 = atomicrmw sub ptr %1087, i64 1 release, align 8, !noalias !23690
  %1089 = icmp eq i64 %1088, 1
  br i1 %1089, label %1090, label %1094

1090:                                             ; preds = %1086
  fence acquire, !noalias !23688
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %144) #91
          to label %1094 unwind label %1091

1091:                                             ; preds = %1090
  %1092 = landingpad { ptr, i32 }
          cleanup
  %1093 = trunc nuw i8 %532 to i1
  br i1 %1093, label %1364, label %1366

1094:                                             ; preds = %1090, %1086
  %1095 = trunc nuw i8 %532 to i1
  br i1 %1095, label %1096, label %1362

1096:                                             ; preds = %1094
  call void @llvm.experimental.noalias.scope.decl(metadata !23691)
  %1097 = getelementptr inbounds nuw i8, ptr %50, i64 8
  %1098 = load ptr, ptr %1097, align 8, !alias.scope !23691, !nonnull !1733, !noundef !1733
  %1099 = load i64, ptr %180, align 8, !alias.scope !23691, !noundef !1733
  call void @llvm.experimental.noalias.scope.decl(metadata !23694)
  %1100 = icmp eq i64 %1099, 0
  br i1 %1100, label %.loopexit69, label %.preheader68

.preheader68:                                     ; preds = %1096
  %1101 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1102 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  br label %1103

1103:                                             ; preds = %.preheader68, %1141
  %1104 = phi i64 [ %1106, %1141 ], [ 0, %.preheader68 ]
  %1105 = getelementptr inbounds nuw [40 x i8], ptr %1098, i64 %1104
  %1106 = add nuw nsw i64 %1104, 1
  %1107 = load i64, ptr %1105, align 8, !range !1771, !alias.scope !23697, !noalias !23691, !noundef !1733
  %1108 = icmp ugt i64 %1107, 5
  br i1 %1108, label %1109, label %1141

1109:                                             ; preds = %1103
  %1110 = getelementptr i8, ptr %1105, i64 8
  %1111 = load ptr, ptr %1110, align 8, !alias.scope !23694, !noalias !23691, !nonnull !1733, !noundef !1733
  %1112 = shl i64 %1107, 3
  %1113 = add i64 %1112, -8
  %1114 = load i64, ptr %1101, align 8, !noalias !23700, !noundef !1733
  %1115 = call i64 @llvm.umin.i64(i64 %1113, i64 9223372036854775807)
  %1116 = call i64 @llvm.ssub.sat.i64(i64 %1114, i64 %1115)
  store i64 %1116, ptr %1101, align 8, !noalias !23700
  %1117 = load i64, ptr %1102, align 8, !noalias !23700, !noundef !1733
  %1118 = icmp slt i64 %1116, %1117
  br i1 %1118, label %1119, label %.preheader404

1119:                                             ; preds = %1109
  store i64 %1116, ptr %1102, align 8, !noalias !23700
  br label %.preheader404

.preheader404:                                    ; preds = %1119, %1109
  br label %1120

1120:                                             ; preds = %.preheader404, %1123
  %1121 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !23700
  %1122 = icmp slt i64 %1121, 0
  br i1 %1122, label %1123, label %__rustc::__rust_dealloc (.exit63)

1123:                                             ; preds = %1120
  %1124 = add nsw i64 %1121, 1
  %1125 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %1121, i64 %1124 acq_rel acquire, align 8, !noalias !23700
  %1126 = extractvalue { i64, i1 } %1125, 1
  br i1 %1126, label %1127, label %1120

1127:                                             ; preds = %1123
  %1128 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1115 monotonic, align 8, !noalias !23700
  %1129 = call i64 @llvm.ssub.sat.i64(i64 %1128, i64 %1115)
  %1130 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !23700
  br label %1131

1131:                                             ; preds = %1134, %1127
  %1132 = phi i64 [ %1130, %1127 ], [ %1137, %1134 ]
  %1133 = icmp slt i64 %1129, %1132
  br i1 %1133, label %1134, label %1138

1134:                                             ; preds = %1131
  %1135 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1132, i64 %1129 monotonic monotonic, align 8, !noalias !23700
  %1136 = extractvalue { i64, i1 } %1135, 1
  %1137 = extractvalue { i64, i1 } %1135, 0
  br i1 %1136, label %1138, label %1131

1138:                                             ; preds = %1134, %1131
  %1139 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !23700
  br label %__rustc::__rust_dealloc (.exit63)

__rustc::__rust_dealloc (.exit63): ; preds = %1120, %1138
  %1140 = icmp ne i64 %1113, 0
  call void @llvm.assume(i1 %1140), !noalias !23700
  call void @free(ptr noundef nonnull %1111) #92, !noalias !23700
  br label %1141

1141:                                             ; preds = %__rustc::__rust_dealloc (.exit63), %1103
  %1142 = icmp eq i64 %1106, %1099
  br i1 %1142, label %.loopexit69, label %1103

.loopexit69:                                      ; preds = %1141, %1096
  %1143 = load i64, ptr %50, align 8, !alias.scope !23691
  %1144 = icmp eq i64 %1143, 0
  br i1 %1144, label %1362, label %1330

1145:                                             ; preds = %997
  %1146 = landingpad { ptr, i32 }
          cleanup
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %42) #89, !noalias !23478
  br label %522

1147:                                             ; preds = %927, %589
  call void @llvm.experimental.noalias.scope.decl(metadata !23703)
  %1148 = load ptr, ptr %41, align 8, !alias.scope !23703, !noalias !23478, !noundef !1733
  %1149 = icmp eq ptr %1148, null
  br i1 %1149, label %1158, label %1150

1150:                                             ; preds = %1147
  %1151 = atomicrmw sub ptr %1148, i64 1 release, align 8, !noalias !23706
  %1152 = icmp eq i64 %1151, 1
  br i1 %1152, label %1153, label %1158

1153:                                             ; preds = %1150
  fence acquire, !noalias !23478
; invoke <alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %41) #91
          to label %1158 unwind label %526, !inline_history !2018

1154:                                             ; preds = %619
  %1155 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>>(ptr noalias nofree noundef align 8 dereferenceable(32) %36) #89
          to label %564 unwind label %543, !noalias !23478

1156:                                             ; preds = %595
  %1157 = landingpad { ptr, i32 }
          cleanup
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %29) #89
  br label %564

1158:                                             ; preds = %1153, %1150, %1147
  call void @llvm.lifetime.end.p0(ptr nonnull %41), !noalias !23472
  br label %1081

1159:                                             ; preds = %1081
  call void @llvm.lifetime.end.p0(ptr nonnull %43), !noalias !23472
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(200) %44)
          to label %1160 unwind label %152

1160:                                             ; preds = %1159
  call void @llvm.lifetime.end.p0(ptr nonnull %44), !noalias !23472
  call void @llvm.experimental.noalias.scope.decl(metadata !23709)
  call void @llvm.experimental.noalias.scope.decl(metadata !23712), !noalias !23478
  %1161 = load ptr, ptr %45, align 8, !alias.scope !23715, !noalias !23478, !nonnull !1733, !noundef !1733
  %1162 = atomicrmw sub ptr %1161, i64 1 release, align 8, !noalias !23716
  %1163 = icmp eq i64 %1162, 1
  br i1 %1163, label %1164, label %1167

1164:                                             ; preds = %1160
  fence acquire, !noalias !23478
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %45) #91
          to label %1167 unwind label %1165

1165:                                             ; preds = %1164
  %1166 = landingpad { ptr, i32 }
          cleanup
  br label %1323

1167:                                             ; preds = %1164, %1160
  call void @llvm.lifetime.end.p0(ptr nonnull %45), !noalias !23472
  call void @llvm.experimental.noalias.scope.decl(metadata !23717)
  %1168 = getelementptr inbounds nuw i8, ptr %47, i64 72
  %1169 = load i64, ptr %1168, align 8, !range !1771, !alias.scope !23720, !noalias !23650, !noundef !1733
  %1170 = icmp ugt i64 %1169, 5
  br i1 %1170, label %1171, label %1205

1171:                                             ; preds = %1167
  %1172 = getelementptr inbounds nuw i8, ptr %47, i64 80
  %1173 = load ptr, ptr %1172, align 8, !alias.scope !23717, !noalias !23650, !nonnull !1733, !noundef !1733
  %1174 = mul i64 %1169, 3
  %1175 = add i64 %1174, -3
  %1176 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1177 = load i64, ptr %1176, align 8, !noalias !23723, !noundef !1733
  %1178 = call i64 @llvm.umin.i64(i64 %1175, i64 9223372036854775807)
  %1179 = call i64 @llvm.ssub.sat.i64(i64 %1177, i64 %1178)
  store i64 %1179, ptr %1176, align 8, !noalias !23723
  %1180 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1181 = load i64, ptr %1180, align 8, !noalias !23723, !noundef !1733
  %1182 = icmp slt i64 %1179, %1181
  br i1 %1182, label %1183, label %.preheader403

1183:                                             ; preds = %1171
  store i64 %1179, ptr %1180, align 8, !noalias !23723
  br label %.preheader403

.preheader403:                                    ; preds = %1183, %1171
  br label %1184

1184:                                             ; preds = %.preheader403, %1187
  %1185 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !23723
  %1186 = icmp slt i64 %1185, 0
  br i1 %1186, label %1187, label %__rustc::__rust_dealloc (.exit64)

1187:                                             ; preds = %1184
  %1188 = add nsw i64 %1185, 1
  %1189 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %1185, i64 %1188 acq_rel acquire, align 8, !noalias !23723
  %1190 = extractvalue { i64, i1 } %1189, 1
  br i1 %1190, label %1191, label %1184

1191:                                             ; preds = %1187
  %1192 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1178 monotonic, align 8, !noalias !23723
  %1193 = call i64 @llvm.ssub.sat.i64(i64 %1192, i64 %1178)
  %1194 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !23723
  br label %1195

1195:                                             ; preds = %1198, %1191
  %1196 = phi i64 [ %1194, %1191 ], [ %1201, %1198 ]
  %1197 = icmp slt i64 %1193, %1196
  br i1 %1197, label %1198, label %1202

1198:                                             ; preds = %1195
  %1199 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1196, i64 %1193 monotonic monotonic, align 8, !noalias !23723
  %1200 = extractvalue { i64, i1 } %1199, 1
  %1201 = extractvalue { i64, i1 } %1199, 0
  br i1 %1200, label %1202, label %1195

1202:                                             ; preds = %1198, %1195
  %1203 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !23723
  br label %__rustc::__rust_dealloc (.exit64)

__rustc::__rust_dealloc (.exit64): ; preds = %1184, %1202
  %1204 = icmp ne i64 %1175, 0
  call void @llvm.assume(i1 %1204), !noalias !23723
  call void @free(ptr noundef nonnull %1173) #92, !noalias !23723
  br label %1205

1205:                                             ; preds = %__rustc::__rust_dealloc (.exit64), %1167
  %1206 = load i64, ptr %47, align 8, !range !2052, !alias.scope !23717, !noalias !23650, !noundef !1733
  %1207 = icmp sgt i64 %1206, 0
  br i1 %1207, label %1208, label %1240

1208:                                             ; preds = %1205
  %1209 = getelementptr inbounds nuw i8, ptr %47, i64 8
  %1210 = load ptr, ptr %1209, align 8, !alias.scope !23717, !noalias !23650, !nonnull !1733, !noundef !1733
  %1211 = mul nuw i64 %1206, 3
  %1212 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1213 = load i64, ptr %1212, align 8, !noalias !23726, !noundef !1733
  %1214 = call i64 @llvm.umin.i64(i64 %1211, i64 9223372036854775807)
  %1215 = call i64 @llvm.ssub.sat.i64(i64 %1213, i64 %1214)
  store i64 %1215, ptr %1212, align 8, !noalias !23726
  %1216 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1217 = load i64, ptr %1216, align 8, !noalias !23726, !noundef !1733
  %1218 = icmp slt i64 %1215, %1217
  br i1 %1218, label %1219, label %.preheader402

1219:                                             ; preds = %1208
  store i64 %1215, ptr %1216, align 8, !noalias !23726
  br label %.preheader402

.preheader402:                                    ; preds = %1219, %1208
  br label %1220

1220:                                             ; preds = %.preheader402, %1223
  %1221 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !23726
  %1222 = icmp slt i64 %1221, 0
  br i1 %1222, label %1223, label %__rustc::__rust_dealloc (.exit65)

1223:                                             ; preds = %1220
  %1224 = add nsw i64 %1221, 1
  %1225 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %1221, i64 %1224 acq_rel acquire, align 8, !noalias !23726
  %1226 = extractvalue { i64, i1 } %1225, 1
  br i1 %1226, label %1227, label %1220

1227:                                             ; preds = %1223
  %1228 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1214 monotonic, align 8, !noalias !23726
  %1229 = call i64 @llvm.ssub.sat.i64(i64 %1228, i64 %1214)
  %1230 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !23726
  br label %1231

1231:                                             ; preds = %1234, %1227
  %1232 = phi i64 [ %1230, %1227 ], [ %1237, %1234 ]
  %1233 = icmp slt i64 %1229, %1232
  br i1 %1233, label %1234, label %1238

1234:                                             ; preds = %1231
  %1235 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1232, i64 %1229 monotonic monotonic, align 8, !noalias !23726
  %1236 = extractvalue { i64, i1 } %1235, 1
  %1237 = extractvalue { i64, i1 } %1235, 0
  br i1 %1236, label %1238, label %1231

1238:                                             ; preds = %1234, %1231
  %1239 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !23726
  br label %__rustc::__rust_dealloc (.exit65)

__rustc::__rust_dealloc (.exit65): ; preds = %1220, %1238
  call void @free(ptr noundef nonnull %1210) #92, !noalias !23726
  br label %1240

1240:                                             ; preds = %__rustc::__rust_dealloc (.exit65), %1205
  %1241 = getelementptr inbounds nuw i8, ptr %47, i64 96
  call void @llvm.experimental.noalias.scope.decl(metadata !23727), !noalias !23650
  %1242 = load ptr, ptr %1241, align 8, !alias.scope !23730, !noalias !23650, !noundef !1733
  %1243 = icmp eq ptr %1242, null
  br i1 %1243, label %1257, label %1244

1244:                                             ; preds = %1240
  %1245 = atomicrmw sub ptr %1242, i64 1 release, align 8, !noalias !23731
  %1246 = icmp eq i64 %1245, 1
  br i1 %1246, label %1247, label %1257

1247:                                             ; preds = %1244
  fence acquire, !noalias !23650
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %1241) #91
          to label %1257 unwind label %1255

1248:                                             ; preds = %1323, %1255, %155, %151
  %1249 = phi i8 [ %1082, %1255 ], [ %1324, %1323 ], [ %186, %151 ], [ %532, %155 ]
  %1250 = phi { ptr, i32 } [ %1256, %1255 ], [ %1325, %1323 ], [ %188, %151 ], [ %156, %155 ]
  call void @llvm.experimental.noalias.scope.decl(metadata !23736)
  call void @llvm.experimental.noalias.scope.decl(metadata !23739), !noalias !23467
  %1251 = load ptr, ptr %144, align 8, !alias.scope !23742, !noalias !23467, !nonnull !1733, !noundef !1733
  %1252 = atomicrmw sub ptr %1251, i64 1 release, align 8, !noalias !23743
  %1253 = icmp eq i64 %1252, 1
  br i1 %1253, label %1254, label %1326

1254:                                             ; preds = %1248
  fence acquire, !noalias !23467
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %144) #91
          to label %1326 unwind label %543

1255:                                             ; preds = %1247
  %1256 = landingpad { ptr, i32 }
          cleanup
  br label %1248

1257:                                             ; preds = %1247, %1244, %1240
  call void @llvm.experimental.noalias.scope.decl(metadata !23744)
  call void @llvm.experimental.noalias.scope.decl(metadata !23747), !noalias !23467
  %1258 = load ptr, ptr %144, align 8, !alias.scope !23750, !noalias !23467, !nonnull !1733, !noundef !1733
  %1259 = atomicrmw sub ptr %1258, i64 1 release, align 8, !noalias !23751
  %1260 = icmp eq i64 %1259, 1
  br i1 %1260, label %1261, label %1265

1261:                                             ; preds = %1257
  fence acquire, !noalias !23467
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %144) #91
          to label %1265 unwind label %1262

1262:                                             ; preds = %1261
  %1263 = landingpad { ptr, i32 }
          cleanup
  %1264 = trunc nuw i8 %1082 to i1
  br i1 %1264, label %1364, label %1366

1265:                                             ; preds = %1261, %1257
  %1266 = trunc nuw i8 %1082 to i1
  br i1 %1266, label %1267, label %1362

1267:                                             ; preds = %1265
  call void @llvm.experimental.noalias.scope.decl(metadata !23752)
  %1268 = getelementptr inbounds nuw i8, ptr %50, i64 8
  %1269 = load ptr, ptr %1268, align 8, !alias.scope !23752, !nonnull !1733, !noundef !1733
  %1270 = load i64, ptr %180, align 8, !alias.scope !23752, !noundef !1733
  call void @llvm.experimental.noalias.scope.decl(metadata !23755)
  %1271 = icmp eq i64 %1270, 0
  br i1 %1271, label %.loopexit, label %.preheader

.preheader:                                       ; preds = %1267
  %1272 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1273 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  br label %1274

1274:                                             ; preds = %.preheader, %1312
  %1275 = phi i64 [ %1277, %1312 ], [ 0, %.preheader ]
  %1276 = getelementptr inbounds nuw [40 x i8], ptr %1269, i64 %1275
  %1277 = add nuw nsw i64 %1275, 1
  %1278 = load i64, ptr %1276, align 8, !range !1771, !alias.scope !23758, !noalias !23752, !noundef !1733
  %1279 = icmp ugt i64 %1278, 5
  br i1 %1279, label %1280, label %1312

1280:                                             ; preds = %1274
  %1281 = getelementptr i8, ptr %1276, i64 8
  %1282 = load ptr, ptr %1281, align 8, !alias.scope !23755, !noalias !23752, !nonnull !1733, !noundef !1733
  %1283 = shl i64 %1278, 3
  %1284 = add i64 %1283, -8
  %1285 = load i64, ptr %1272, align 8, !noalias !23761, !noundef !1733
  %1286 = call i64 @llvm.umin.i64(i64 %1284, i64 9223372036854775807)
  %1287 = call i64 @llvm.ssub.sat.i64(i64 %1285, i64 %1286)
  store i64 %1287, ptr %1272, align 8, !noalias !23761
  %1288 = load i64, ptr %1273, align 8, !noalias !23761, !noundef !1733
  %1289 = icmp slt i64 %1287, %1288
  br i1 %1289, label %1290, label %.preheader401

1290:                                             ; preds = %1280
  store i64 %1287, ptr %1273, align 8, !noalias !23761
  br label %.preheader401

.preheader401:                                    ; preds = %1290, %1280
  br label %1291

1291:                                             ; preds = %.preheader401, %1294
  %1292 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !23761
  %1293 = icmp slt i64 %1292, 0
  br i1 %1293, label %1294, label %__rustc::__rust_dealloc (.exit66)

1294:                                             ; preds = %1291
  %1295 = add nsw i64 %1292, 1
  %1296 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %1292, i64 %1295 acq_rel acquire, align 8, !noalias !23761
  %1297 = extractvalue { i64, i1 } %1296, 1
  br i1 %1297, label %1298, label %1291

1298:                                             ; preds = %1294
  %1299 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1286 monotonic, align 8, !noalias !23761
  %1300 = call i64 @llvm.ssub.sat.i64(i64 %1299, i64 %1286)
  %1301 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !23761
  br label %1302

1302:                                             ; preds = %1305, %1298
  %1303 = phi i64 [ %1301, %1298 ], [ %1308, %1305 ]
  %1304 = icmp slt i64 %1300, %1303
  br i1 %1304, label %1305, label %1309

1305:                                             ; preds = %1302
  %1306 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1303, i64 %1300 monotonic monotonic, align 8, !noalias !23761
  %1307 = extractvalue { i64, i1 } %1306, 1
  %1308 = extractvalue { i64, i1 } %1306, 0
  br i1 %1307, label %1309, label %1302

1309:                                             ; preds = %1305, %1302
  %1310 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !23761
  br label %__rustc::__rust_dealloc (.exit66)

__rustc::__rust_dealloc (.exit66): ; preds = %1291, %1309
  %1311 = icmp ne i64 %1284, 0
  call void @llvm.assume(i1 %1311), !noalias !23761
  call void @free(ptr noundef nonnull %1282) #92, !noalias !23761
  br label %1312

1312:                                             ; preds = %__rustc::__rust_dealloc (.exit66), %1274
  %1313 = icmp eq i64 %1277, %1270
  br i1 %1313, label %.loopexit, label %1274

.loopexit:                                        ; preds = %1312, %1267
  %1314 = load i64, ptr %50, align 8, !alias.scope !23752
  %1315 = icmp eq i64 %1314, 0
  br i1 %1315, label %1362, label %1330

1316:                                             ; preds = %152, %151
  %1317 = phi { ptr, i32 } [ %154, %152 ], [ %188, %151 ]
  %1318 = phi i8 [ %153, %152 ], [ %186, %151 ]
  call void @llvm.experimental.noalias.scope.decl(metadata !23764)
  call void @llvm.experimental.noalias.scope.decl(metadata !23767), !noalias !23478
  %1319 = load ptr, ptr %45, align 8, !alias.scope !23770, !noalias !23478, !nonnull !1733, !noundef !1733
  %1320 = atomicrmw sub ptr %1319, i64 1 release, align 8, !noalias !23771
  %1321 = icmp eq i64 %1320, 1
  br i1 %1321, label %1322, label %1323

1322:                                             ; preds = %1316
  fence acquire, !noalias !23478
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %45) #91
          to label %1323 unwind label %543

1323:                                             ; preds = %1322, %1316, %1165
  %1324 = phi i8 [ %1082, %1165 ], [ %1318, %1322 ], [ %1318, %1316 ]
  %1325 = phi { ptr, i32 } [ %1166, %1165 ], [ %1317, %1322 ], [ %1317, %1316 ]
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>(ptr noalias nofree noundef nonnull align 8 dereferenceable(104) %47) #89
          to label %1248 unwind label %543, !noalias !23650

1326:                                             ; preds = %1254, %1248
  %1327 = trunc nuw i8 %1249 to i1
  br i1 %1327, label %1364, label %1366

1328:                                             ; preds = %140
  call void @llvm.lifetime.end.p0(ptr nonnull %48)
  call void @llvm.lifetime.end.p0(ptr nonnull %49)
  %1329 = getelementptr inbounds nuw i8, ptr %0, i64 8
; call <purrdf_sparql_eval::governor::lift::Lift>::withheld::<purrdf_core::ir::term::TermId>
  call fastcc void @<purrdf_sparql_eval::governor::lift::Lift>::withheld::<purrdf_core::ir::term::TermId>(ptr noalias nofree noundef align 8 captures(address) dereferenceable(96) %1329, ptr noalias nofree noundef align 8 captures(address) dereferenceable(104) %51)
  store i64 0, ptr %0, align 16
  br label %1363

1330:                                             ; preds = %.loopexit, %.loopexit69
  %1331 = phi i64 [ %1143, %.loopexit69 ], [ %1314, %.loopexit ]
  %1332 = phi ptr [ %1098, %.loopexit69 ], [ %1269, %.loopexit ]
  %1333 = mul nuw i64 %1331, 40
  %1334 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1335 = load i64, ptr %1334, align 8, !noalias !1733, !noundef !1733
  %1336 = call i64 @llvm.umin.i64(i64 %1333, i64 9223372036854775807)
  %1337 = call i64 @llvm.ssub.sat.i64(i64 %1335, i64 %1336)
  store i64 %1337, ptr %1334, align 8, !noalias !1733
  %1338 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1339 = load i64, ptr %1338, align 8, !noalias !1733, !noundef !1733
  %1340 = icmp slt i64 %1337, %1339
  br i1 %1340, label %1341, label %.preheader400

1341:                                             ; preds = %1330
  store i64 %1337, ptr %1338, align 8, !noalias !1733
  br label %.preheader400

.preheader400:                                    ; preds = %1341, %1330
  br label %1342

1342:                                             ; preds = %.preheader400, %1345
  %1343 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !1733
  %1344 = icmp slt i64 %1343, 0
  br i1 %1344, label %1345, label %__rustc::__rust_dealloc (.exit67)

1345:                                             ; preds = %1342
  %1346 = add nsw i64 %1343, 1
  %1347 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %1343, i64 %1346 acq_rel acquire, align 8, !noalias !1733
  %1348 = extractvalue { i64, i1 } %1347, 1
  br i1 %1348, label %1349, label %1342

1349:                                             ; preds = %1345
  %1350 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1336 monotonic, align 8, !noalias !1733
  %1351 = call i64 @llvm.ssub.sat.i64(i64 %1350, i64 %1336)
  %1352 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !1733
  br label %1353

1353:                                             ; preds = %1356, %1349
  %1354 = phi i64 [ %1352, %1349 ], [ %1359, %1356 ]
  %1355 = icmp slt i64 %1351, %1354
  br i1 %1355, label %1356, label %1360

1356:                                             ; preds = %1353
  %1357 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1354, i64 %1351 monotonic monotonic, align 8, !noalias !1733
  %1358 = extractvalue { i64, i1 } %1357, 1
  %1359 = extractvalue { i64, i1 } %1357, 0
  br i1 %1358, label %1360, label %1353

1360:                                             ; preds = %1356, %1353
  %1361 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !1733
  br label %__rustc::__rust_dealloc (.exit67)

__rustc::__rust_dealloc (.exit67): ; preds = %1342, %1360
  call void @free(ptr noundef nonnull %1332) #92, !noalias !1733
  br label %1362

1362:                                             ; preds = %__rustc::__rust_dealloc (.exit67), %.loopexit, %1265, %.loopexit69, %1094
  call void @llvm.lifetime.end.p0(ptr nonnull %47)
  call void @llvm.lifetime.end.p0(ptr nonnull %50)
  br label %1363

1363:                                             ; preds = %1362, %1328, %137, %134, %130
  ret void

1364:                                             ; preds = %1326, %1262, %1091
  %1365 = phi { ptr, i32 } [ %1092, %1091 ], [ %1263, %1262 ], [ %1250, %1326 ]
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef nonnull align 8 dereferenceable(32) %50) #89, !noalias !23467
  br label %1366

1366:                                             ; preds = %1368, %1364, %1326, %1262, %1091
  %1367 = phi { ptr, i32 } [ %1092, %1091 ], [ %1369, %1368 ], [ %1250, %1326 ], [ %1263, %1262 ], [ %1365, %1364 ]
  resume { ptr, i32 } %1367

1368:                                             ; preds = %138, %5
  %1369 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>(ptr noalias nofree noundef align 8 dereferenceable(104) %51) #89
          to label %1366 unwind label %1370

1370:                                             ; preds = %1368
  %1371 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  tail call void @core::panicking::panic_in_cleanup() #90
  unreachable
}

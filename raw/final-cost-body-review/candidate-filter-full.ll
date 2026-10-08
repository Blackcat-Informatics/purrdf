define void @purrdf_sparql_eval::expr::eval_filter::<purrdf_core::ir::dataset::RdfDataset>(ptr dead_on_unwind noalias nofree noundef writable writeonly sret([112 x i8]) align 16 captures(none) dereferenceable(112) %0, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(144) %1, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(64) %2, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(144) %3, ptr noalias nofree noundef align 16 dereferenceable(1248) %4) unnamed_addr #8 personality ptr @rust_eh_personality !guid !23443 {
  %6 = alloca [96 x i8], align 16
  %7 = alloca [96 x i8], align 16
  %8 = alloca [96 x i8], align 16
  %9 = alloca [96 x i8], align 16
  %10 = alloca [40 x i8], align 8
  %11 = alloca [24 x i8], align 8
  %12 = alloca [48 x i8], align 8
  %13 = alloca [24 x i8], align 8
  %14 = alloca [24 x i8], align 8
  %15 = alloca [32 x i8], align 8
  %16 = alloca [96 x i8], align 16
  %17 = alloca [24 x i8], align 8
  %18 = alloca [32 x i8], align 8
  %19 = alloca [224 x i8], align 8
  %20 = alloca [24 x i8], align 8
  %21 = alloca [32 x i8], align 8
  %22 = alloca [104 x i8], align 8
  %23 = alloca [96 x i8], align 8
  %24 = alloca [96 x i8], align 8
  %25 = alloca [24 x i8], align 8
  %26 = alloca [24 x i8], align 8
  %27 = alloca [80 x i8], align 16
  %28 = alloca [24 x i8], align 8
  %29 = alloca [40 x i8], align 8
  %30 = alloca [32 x i8], align 8
  %31 = alloca [32 x i8], align 8
  %32 = alloca [24 x i8], align 8
  %33 = alloca [16 x i8], align 8
  %34 = alloca [80 x i8], align 16
  %35 = alloca [24 x i8], align 8
  %36 = alloca [200 x i8], align 8
  %37 = alloca [24 x i8], align 8
  %38 = alloca [24 x i8], align 8
  %39 = alloca [24 x i8], align 8
  %40 = alloca [248 x i8], align 8
  %41 = alloca [240 x i8], align 8
  %42 = alloca [208 x i8], align 8
  %43 = alloca [32 x i8], align 8
  %44 = alloca [24 x i8], align 8
  %45 = alloca [32 x i8], align 8
  %46 = alloca [256 x i8], align 16
  %47 = alloca [208 x i8], align 16
  %48 = alloca [8 x i8], align 8
  %49 = alloca [24 x i8], align 8
  %50 = alloca [216 x i8], align 8
  %51 = alloca [200 x i8], align 8
  %52 = alloca [8 x i8], align 8
  %53 = alloca [112 x i8], align 16
  %54 = alloca [104 x i8], align 8
  %55 = alloca [104 x i8], align 8
  %56 = alloca [32 x i8], align 8
  %57 = alloca [32 x i8], align 8
  %58 = alloca [104 x i8], align 8
; call <purrdf_sparql_eval::governor::lift::Lift>::at
  call void @<purrdf_sparql_eval::governor::lift::Lift>::at(ptr noalias nofree noundef nonnull sret([104 x i8]) align 8 captures(none) dereferenceable(104) %58, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %1)
  call void @llvm.lifetime.start.p0(ptr nonnull %56)
  call void @llvm.lifetime.start.p0(ptr nonnull %55)
; invoke purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
  invoke fastcc void @purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>(ptr noalias nofree noundef nonnull align 16 captures(none) dereferenceable(112) %53, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %3, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %4, ptr noundef nonnull readonly align 8 dereferenceable(144) %3)
          to label %59 unwind label %1576, !inline_history !13429

59:                                               ; preds = %5
  %60 = load i64, ptr %53, align 16, !range !1739, !noundef !1740
  %61 = trunc nuw i64 %60 to i1
  br i1 %61, label %62, label %145

62:                                               ; preds = %59
  %63 = getelementptr inbounds nuw i8, ptr %53, i64 16
  %64 = getelementptr inbounds nuw i8, ptr %0, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(96) %64, ptr noundef nonnull align 16 dereferenceable(96) %63, i64 96, i1 false)
  store i64 1, ptr %0, align 16
  call void @llvm.lifetime.end.p0(ptr nonnull %55)
  call void @llvm.lifetime.end.p0(ptr nonnull %56)
  %65 = getelementptr inbounds nuw i8, ptr %58, i64 72
  %66 = load i64, ptr %65, align 8, !range !1778, !noundef !1740
  %67 = icmp ugt i64 %66, 5
  br i1 %67, label %68, label %102

68:                                               ; preds = %62
  %69 = getelementptr inbounds nuw i8, ptr %58, i64 80
  %70 = load ptr, ptr %69, align 8, !nonnull !1740, !noundef !1740
  %71 = mul i64 %66, 3
  %72 = add i64 %71, -3
  %73 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %74 = load i64, ptr %73, align 8, !noalias !23444, !noundef !1740
  %75 = tail call i64 @llvm.umin.i64(i64 %72, i64 9223372036854775807)
  %76 = tail call i64 @llvm.ssub.sat.i64(i64 %74, i64 %75)
  store i64 %76, ptr %73, align 8, !noalias !23444
  %77 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %78 = load i64, ptr %77, align 8, !noalias !23444, !noundef !1740
  %79 = icmp slt i64 %76, %78
  br i1 %79, label %80, label %.preheader473

80:                                               ; preds = %68
  store i64 %76, ptr %77, align 8, !noalias !23444
  br label %.preheader473

.preheader473:                                    ; preds = %80, %68
  br label %81

81:                                               ; preds = %.preheader473, %84
  %82 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !23444
  %83 = icmp slt i64 %82, 0
  br i1 %83, label %84, label %__rustc::__rust_dealloc (.exit)

84:                                               ; preds = %81
  %85 = add nsw i64 %82, 1
  %86 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %82, i64 %85 acq_rel acquire, align 8, !noalias !23444
  %87 = extractvalue { i64, i1 } %86, 1
  br i1 %87, label %88, label %81

88:                                               ; preds = %84
  %89 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %75 monotonic, align 8, !noalias !23444
  %90 = tail call i64 @llvm.ssub.sat.i64(i64 %89, i64 %75)
  %91 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !23444
  br label %92

92:                                               ; preds = %95, %88
  %93 = phi i64 [ %91, %88 ], [ %98, %95 ]
  %94 = icmp slt i64 %90, %93
  br i1 %94, label %95, label %99

95:                                               ; preds = %92
  %96 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %93, i64 %90 monotonic monotonic, align 8, !noalias !23444
  %97 = extractvalue { i64, i1 } %96, 1
  %98 = extractvalue { i64, i1 } %96, 0
  br i1 %97, label %99, label %92

99:                                               ; preds = %95, %92
  %100 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !23444
  br label %__rustc::__rust_dealloc (.exit)

__rustc::__rust_dealloc (.exit): ; preds = %81, %99
  %101 = icmp ne i64 %72, 0
  tail call void @llvm.assume(i1 %101), !noalias !23444
  tail call void @free(ptr noundef nonnull %70) #92, !noalias !23444
  br label %102

102:                                              ; preds = %__rustc::__rust_dealloc (.exit), %62
  %103 = load i64, ptr %58, align 8, !range !2059, !noundef !1740
  %104 = icmp sgt i64 %103, 0
  br i1 %104, label %105, label %137

105:                                              ; preds = %102
  %106 = getelementptr inbounds nuw i8, ptr %58, i64 8
  %107 = load ptr, ptr %106, align 8, !nonnull !1740, !noundef !1740
  %108 = mul nuw i64 %103, 3
  %109 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %110 = load i64, ptr %109, align 8, !noalias !23449, !noundef !1740
  %111 = tail call i64 @llvm.umin.i64(i64 %108, i64 9223372036854775807)
  %112 = tail call i64 @llvm.ssub.sat.i64(i64 %110, i64 %111)
  store i64 %112, ptr %109, align 8, !noalias !23449
  %113 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %114 = load i64, ptr %113, align 8, !noalias !23449, !noundef !1740
  %115 = icmp slt i64 %112, %114
  br i1 %115, label %116, label %.preheader472

116:                                              ; preds = %105
  store i64 %112, ptr %113, align 8, !noalias !23449
  br label %.preheader472

.preheader472:                                    ; preds = %116, %105
  br label %117

117:                                              ; preds = %.preheader472, %120
  %118 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !23449
  %119 = icmp slt i64 %118, 0
  br i1 %119, label %120, label %__rustc::__rust_dealloc (.exit58)

120:                                              ; preds = %117
  %121 = add nsw i64 %118, 1
  %122 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %118, i64 %121 acq_rel acquire, align 8, !noalias !23449
  %123 = extractvalue { i64, i1 } %122, 1
  br i1 %123, label %124, label %117

124:                                              ; preds = %120
  %125 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %111 monotonic, align 8, !noalias !23449
  %126 = tail call i64 @llvm.ssub.sat.i64(i64 %125, i64 %111)
  %127 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !23449
  br label %128

128:                                              ; preds = %131, %124
  %129 = phi i64 [ %127, %124 ], [ %134, %131 ]
  %130 = icmp slt i64 %126, %129
  br i1 %130, label %131, label %135

131:                                              ; preds = %128
  %132 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %129, i64 %126 monotonic monotonic, align 8, !noalias !23449
  %133 = extractvalue { i64, i1 } %132, 1
  %134 = extractvalue { i64, i1 } %132, 0
  br i1 %133, label %135, label %128

135:                                              ; preds = %131, %128
  %136 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !23449
  br label %__rustc::__rust_dealloc (.exit58)

__rustc::__rust_dealloc (.exit58): ; preds = %117, %135
  tail call void @free(ptr noundef nonnull %107) #92, !noalias !23449
  br label %137

137:                                              ; preds = %__rustc::__rust_dealloc (.exit58), %102
  %138 = getelementptr inbounds nuw i8, ptr %58, i64 96
  %139 = load ptr, ptr %138, align 8, !noundef !1740
  %140 = icmp eq ptr %139, null
  br i1 %140, label %1571, label %141

141:                                              ; preds = %137
  %142 = atomicrmw sub ptr %139, i64 1 release, align 8, !noalias !23450
  %143 = icmp eq i64 %142, 1
  br i1 %143, label %144, label %1571

144:                                              ; preds = %141
  fence acquire
; call <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %138) #91
  br label %1571

145:                                              ; preds = %59
  %146 = getelementptr inbounds nuw i8, ptr %53, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(96) %55, ptr noundef nonnull align 8 dereferenceable(96) %146, i64 96, i1 false)
; invoke <purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
  invoke fastcc void @<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>(ptr noalias nofree noundef align 8 captures(none) dereferenceable(32) %56, ptr noalias nofree noundef align 8 dereferenceable(104) %58, i64 noundef 0, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(96) %55)
          to label %147 unwind label %1576

147:                                              ; preds = %145
  %148 = load i64, ptr %56, align 8, !range !2059, !noundef !1740
  %149 = icmp eq i64 %148, -1
  br i1 %149, label %1536, label %150

150:                                              ; preds = %147
  call void @llvm.lifetime.start.p0(ptr nonnull %57)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %57, ptr noundef nonnull align 8 dereferenceable(32) %56, i64 32, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %55)
  call void @llvm.lifetime.end.p0(ptr nonnull %56)
  call void @llvm.lifetime.start.p0(ptr nonnull %54)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(104) %54, ptr noundef nonnull align 8 dereferenceable(104) %58, i64 104, i1 false)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !23457)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !23460)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !23462)
  call void @llvm.lifetime.start.p0(ptr nonnull %52), !noalias !23464
  %151 = getelementptr inbounds nuw i8, ptr %57, i64 24
  %152 = load ptr, ptr %151, align 8, !alias.scope !23460, !noalias !23468, !nonnull !1740, !noundef !1740
  %153 = atomicrmw add ptr %152, i64 1 monotonic, align 8, !noalias !23464
  %154 = icmp slt i64 %153, 0
  br i1 %154, label %157, label %155

155:                                              ; preds = %150
  store ptr %152, ptr %52, align 8, !noalias !23464
; invoke <purrdf_sparql_eval::eval::EvalCtx>::may_fork_row_loop
  %156 = invoke fastcc noundef zeroext i1 @<purrdf_sparql_eval::eval::EvalCtx>::may_fork_row_loop(ptr noundef nonnull align 16 dereferenceable(1248) %4, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(64) %2)
          to label %164 unwind label %159, !noalias !23469

157:                                              ; preds = %150
  tail call void @llvm.trap()
  unreachable

158:                                              ; preds = %192
  br i1 %194, label %1524, label %1456

159:                                              ; preds = %1367, %185, %155
  %160 = phi i8 [ 1, %155 ], [ 1, %185 ], [ %1290, %1367 ]
  %161 = landingpad { ptr, i32 }
          cleanup
  br label %1524

162:                                              ; preds = %1293
  %163 = landingpad { ptr, i32 }
          cleanup
  br label %1456

164:                                              ; preds = %155
  %165 = getelementptr inbounds nuw i8, ptr %4, i64 472
  %166 = load i8, ptr %165, align 8, !range !3730
  %167 = icmp eq i8 %166, 2
  %168 = select i1 %156, i1 %167, i1 false
  br i1 %168, label %169, label %185

169:                                              ; preds = %164
  %170 = getelementptr inbounds nuw i8, ptr %4, i64 616
  %171 = load ptr, ptr %170, align 8, !noalias !23469, !noundef !1740
  %172 = icmp eq ptr %171, null
  br i1 %172, label %185, label %173

173:                                              ; preds = %169
  %174 = getelementptr inbounds nuw i8, ptr %171, i64 24
  %175 = load i64, ptr %174, align 8, !noalias !23470
  %176 = getelementptr inbounds nuw i8, ptr %171, i64 48
  %177 = icmp ult i64 %175, -2
  br i1 %177, label %185, label %178

178:                                              ; preds = %173
  %179 = getelementptr inbounds nuw i8, ptr %171, i64 32
  %180 = load i64, ptr %179, align 8, !noalias !23470
  %181 = icmp ult i64 %180, -2
  br i1 %181, label %185, label %182

182:                                              ; preds = %178
  %183 = load i64, ptr %176, align 8, !noalias !23470
  %184 = icmp ugt i64 %183, -3
  br label %185

185:                                              ; preds = %182, %178, %173, %169, %164
  %186 = phi i1 [ false, %164 ], [ false, %173 ], [ true, %169 ], [ false, %178 ], [ %184, %182 ]
  call void @llvm.lifetime.start.p0(ptr nonnull %51), !noalias !23464
  %187 = getelementptr inbounds nuw i8, ptr %57, i64 16
  %188 = load i64, ptr %187, align 8, !alias.scope !23460, !noalias !23468, !noundef !1740
  %189 = icmp ult i64 %188, 230584300921369396
  tail call void @llvm.assume(i1 %189)
; invoke <purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::for_rows::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::for_rows::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef align 8 captures(none) dereferenceable(200) %51, ptr noundef nonnull align 16 dereferenceable(1248) %4, i1 noundef zeroext %186, i64 noundef %188)
          to label %190 unwind label %159

190:                                              ; preds = %185
; invoke purrdf_sparql_eval::vm::program_at::<purrdf_core::ir::dataset::RdfDataset>
  %191 = invoke fastcc noundef nonnull ptr @purrdf_sparql_eval::vm::program_at::<purrdf_core::ir::dataset::RdfDataset>(ptr noundef nonnull align 16 dereferenceable(1248) %4, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %1, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(64) %2)
          to label %200 unwind label %196, !noalias !23469

192:                                              ; preds = %529, %196
  %193 = phi i8 [ %197, %196 ], [ %530, %529 ]
  %194 = phi i1 [ %198, %196 ], [ %531, %529 ]
  %195 = phi { ptr, i32 } [ %199, %196 ], [ %532, %529 ]
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(200) %51)
          to label %158 unwind label %550

196:                                              ; preds = %1291, %1289, %200, %190
  %197 = phi i8 [ %1290, %1289 ], [ %539, %1291 ], [ 1, %200 ], [ 1, %190 ]
  %198 = phi i1 [ true, %1289 ], [ false, %1291 ], [ true, %200 ], [ true, %190 ]
  %199 = landingpad { ptr, i32 }
          cleanup
  br label %192

200:                                              ; preds = %190
  call void @llvm.lifetime.start.p0(ptr nonnull %50), !noalias !23464
  %201 = load ptr, ptr %52, align 8, !noalias !23464, !nonnull !1740, !noundef !1740
  %202 = getelementptr inbounds nuw i8, ptr %201, i64 16
; invoke <purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::link::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::link::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef align 8 captures(address) dereferenceable(216) %50, ptr noundef nonnull %191, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(64) %2, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(56) %202, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %4)
          to label %203 unwind label %196, !noalias !23469

203:                                              ; preds = %200
  call void @llvm.lifetime.start.p0(ptr nonnull %49), !noalias !23464
  br i1 %186, label %552, label %204

204:                                              ; preds = %203
  call void @llvm.lifetime.start.p0(ptr nonnull %32), !noalias !23464
  store i64 0, ptr %32, align 8, !noalias !23464
  %205 = getelementptr inbounds nuw i8, ptr %32, i64 8
  store ptr inttoptr (i64 8 to ptr), ptr %205, align 8, !noalias !23464
  %206 = getelementptr inbounds nuw i8, ptr %32, i64 16
  store i64 0, ptr %206, align 8, !noalias !23464
  %207 = getelementptr inbounds nuw i8, ptr %57, i64 8
  %208 = load ptr, ptr %207, align 8, !alias.scope !23460, !noalias !23468, !nonnull !1740, !noundef !1740
  %209 = load i64, ptr %57, align 8, !range !1835, !alias.scope !23460, !noalias !23468, !noundef !1740
  %210 = mul nuw nsw i64 %188, 40
  %211 = getelementptr inbounds nuw i8, ptr %208, i64 %210
  call void @llvm.lifetime.start.p0(ptr nonnull %31), !noalias !23464
  store ptr %208, ptr %31, align 8, !noalias !23464
  %212 = getelementptr inbounds nuw i8, ptr %31, i64 8
  %213 = getelementptr inbounds nuw i8, ptr %31, i64 16
  store i64 %209, ptr %213, align 8, !noalias !23464
  %214 = getelementptr inbounds nuw i8, ptr %31, i64 24
  store ptr %211, ptr %214, align 8, !noalias !23464
  call void @llvm.lifetime.start.p0(ptr nonnull %30)
  %215 = icmp eq i64 %188, 0
  br i1 %215, label %.loopexit88, label %216

216:                                              ; preds = %204
  %217 = getelementptr inbounds nuw i8, ptr %29, i64 8
  %218 = getelementptr inbounds nuw i8, ptr %29, i64 16
  %219 = getelementptr inbounds nuw i8, ptr %30, i64 8
  %220 = getelementptr inbounds nuw i8, ptr %9, i64 8
  %221 = getelementptr inbounds nuw i8, ptr %9, i64 12
  %222 = getelementptr inbounds nuw i8, ptr %8, i64 8
  %223 = tail call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %224 = tail call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  br label %232

225:                                              ; preds = %548, %424
  %226 = phi ptr [ %549, %548 ], [ %329, %424 ]
  %227 = phi { ptr, i32 } [ %546, %548 ], [ %422, %424 ]
  %228 = shl i64 %238, 3
  %229 = add i64 %228, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %226, i64 noundef %229, i64 noundef range(i64 1, -9223372036854775807) 4) #92, !noalias !1740
  br label %230

230:                                              ; preds = %545, %421, %225
  %231 = phi { ptr, i32 } [ %422, %421 ], [ %546, %545 ], [ %227, %225 ]
; call core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(32) %31) #89, !noalias !23469
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %32) #89, !noalias !23469
  br label %529

232:                                              ; preds = %425, %216
  %233 = phi ptr [ inttoptr (i64 8 to ptr), %216 ], [ %426, %425 ]
  %234 = phi i64 [ 0, %216 ], [ %427, %425 ]
  %235 = phi ptr [ inttoptr (i64 8 to ptr), %216 ], [ %428, %425 ]
  %236 = phi ptr [ %208, %216 ], [ %237, %425 ]
  %237 = getelementptr inbounds nuw i8, ptr %236, i64 40
  %238 = load i64, ptr %236, align 8, !noalias !23476
  %239 = getelementptr inbounds nuw i8, ptr %236, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %30, ptr noundef nonnull align 8 dereferenceable(32) %239, i64 32, i1 false), !noalias !23476
  %240 = icmp eq i64 %238, 0
  br i1 %240, label %.loopexit88, label %241

241:                                              ; preds = %232
  call void @llvm.lifetime.start.p0(ptr nonnull %29), !noalias !23464
  store i64 %238, ptr %29, align 8, !noalias !23464
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %217, ptr noundef nonnull align 8 dereferenceable(32) %30, i64 32, i1 false), !noalias !23464
  call void @llvm.lifetime.start.p0(ptr nonnull %28), !noalias !23464
; invoke <purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::pass::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::pass::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %28, ptr noalias nofree noundef align 8 dereferenceable(200) %51, ptr noundef nonnull align 16 dereferenceable(1248) %4)
          to label %323 unwind label %545, !noalias !23469

.loopexit88:                                      ; preds = %425, %232, %204
  %242 = phi ptr [ %208, %204 ], [ %211, %425 ], [ %237, %232 ]
  store ptr %242, ptr %212, align 8
  br label %243

243:                                              ; preds = %536, %.loopexit88
  %244 = phi ptr [ %237, %536 ], [ %242, %.loopexit88 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %30)
  %245 = ptrtoint ptr %211 to i64
  %246 = ptrtoint ptr %244 to i64
  %247 = sub nuw i64 %245, %246
  %248 = udiv exact i64 %247, 40
  call void @llvm.experimental.noalias.scope.decl(metadata !23479), !noalias !23469
  %249 = icmp eq ptr %211, %244
  br i1 %249, label %.loopexit83, label %.preheader82

.preheader82:                                     ; preds = %243
  %250 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %251 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  br label %252

252:                                              ; preds = %.preheader82, %290
  %253 = phi i64 [ %255, %290 ], [ 0, %.preheader82 ]
  %254 = getelementptr inbounds nuw [40 x i8], ptr %244, i64 %253
  %255 = add nuw nsw i64 %253, 1
  %256 = load i64, ptr %254, align 8, !range !1778, !alias.scope !23482, !noalias !23485, !noundef !1740
  %257 = icmp ugt i64 %256, 5
  br i1 %257, label %258, label %290

258:                                              ; preds = %252
  %259 = getelementptr i8, ptr %254, i64 8
  %260 = load ptr, ptr %259, align 8, !alias.scope !23479, !noalias !23485, !nonnull !1740, !noundef !1740
  %261 = shl i64 %256, 3
  %262 = add i64 %261, -8
  %263 = load i64, ptr %250, align 8, !noalias !23490, !noundef !1740
  %264 = call i64 @llvm.umin.i64(i64 %262, i64 9223372036854775807)
  %265 = call i64 @llvm.ssub.sat.i64(i64 %263, i64 %264)
  store i64 %265, ptr %250, align 8, !noalias !23490
  %266 = load i64, ptr %251, align 8, !noalias !23490, !noundef !1740
  %267 = icmp slt i64 %265, %266
  br i1 %267, label %268, label %.preheader514

268:                                              ; preds = %258
  store i64 %265, ptr %251, align 8, !noalias !23490
  br label %.preheader514

.preheader514:                                    ; preds = %268, %258
  br label %269

269:                                              ; preds = %.preheader514, %272
  %270 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !23490
  %271 = icmp slt i64 %270, 0
  br i1 %271, label %272, label %__rustc::__rust_dealloc (.exit59)

272:                                              ; preds = %269
  %273 = add nsw i64 %270, 1
  %274 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %270, i64 %273 acq_rel acquire, align 8, !noalias !23490
  %275 = extractvalue { i64, i1 } %274, 1
  br i1 %275, label %276, label %269

276:                                              ; preds = %272
  %277 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %264 monotonic, align 8, !noalias !23490
  %278 = call i64 @llvm.ssub.sat.i64(i64 %277, i64 %264)
  %279 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !23490
  br label %280

280:                                              ; preds = %283, %276
  %281 = phi i64 [ %279, %276 ], [ %286, %283 ]
  %282 = icmp slt i64 %278, %281
  br i1 %282, label %283, label %287

283:                                              ; preds = %280
  %284 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %281, i64 %278 monotonic monotonic, align 8, !noalias !23490
  %285 = extractvalue { i64, i1 } %284, 1
  %286 = extractvalue { i64, i1 } %284, 0
  br i1 %285, label %287, label %280

287:                                              ; preds = %283, %280
  %288 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !23490
  br label %__rustc::__rust_dealloc (.exit59)

__rustc::__rust_dealloc (.exit59): ; preds = %269, %287
  %289 = icmp ne i64 %262, 0
  call void @llvm.assume(i1 %289), !noalias !23490
  call void @free(ptr noundef nonnull %260) #92, !noalias !23490
  br label %290

290:                                              ; preds = %__rustc::__rust_dealloc (.exit59), %252
  %291 = icmp eq i64 %255, %248
  br i1 %291, label %.loopexit83, label %252

.loopexit83:                                      ; preds = %290, %243
  %292 = icmp eq i64 %209, 0
  br i1 %292, label %537, label %293

293:                                              ; preds = %.loopexit83
  %294 = mul nuw i64 %209, 40
  %295 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %296 = load i64, ptr %295, align 8, !noalias !23485, !noundef !1740
  %297 = call i64 @llvm.umin.i64(i64 %294, i64 9223372036854775807)
  %298 = call i64 @llvm.ssub.sat.i64(i64 %296, i64 %297)
  store i64 %298, ptr %295, align 8, !noalias !23485
  %299 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %300 = load i64, ptr %299, align 8, !noalias !23485, !noundef !1740
  %301 = icmp slt i64 %298, %300
  br i1 %301, label %302, label %.preheader513

302:                                              ; preds = %293
  store i64 %298, ptr %299, align 8, !noalias !23485
  br label %.preheader513

.preheader513:                                    ; preds = %302, %293
  br label %303

303:                                              ; preds = %.preheader513, %306
  %304 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !23485
  %305 = icmp slt i64 %304, 0
  br i1 %305, label %306, label %__rustc::__rust_dealloc (.exit60)

306:                                              ; preds = %303
  %307 = add nsw i64 %304, 1
  %308 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %304, i64 %307 acq_rel acquire, align 8, !noalias !23485
  %309 = extractvalue { i64, i1 } %308, 1
  br i1 %309, label %310, label %303

310:                                              ; preds = %306
  %311 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %297 monotonic, align 8, !noalias !23485
  %312 = call i64 @llvm.ssub.sat.i64(i64 %311, i64 %297)
  %313 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !23485
  br label %314

314:                                              ; preds = %317, %310
  %315 = phi i64 [ %313, %310 ], [ %320, %317 ]
  %316 = icmp slt i64 %312, %315
  br i1 %316, label %317, label %321

317:                                              ; preds = %314
  %318 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %315, i64 %312 monotonic monotonic, align 8, !noalias !23485
  %319 = extractvalue { i64, i1 } %318, 1
  %320 = extractvalue { i64, i1 } %318, 0
  br i1 %319, label %321, label %314

321:                                              ; preds = %317, %314
  %322 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !23485
  br label %__rustc::__rust_dealloc (.exit60)

__rustc::__rust_dealloc (.exit60): ; preds = %303, %321
  call void @free(ptr noundef nonnull %208) #92, !noalias !23485
  br label %537

323:                                              ; preds = %241
  %324 = load i8, ptr %28, align 8, !range !1743, !noalias !23464, !noundef !1740
  %325 = icmp eq i8 %324, -1
  br i1 %325, label %326, label %360

326:                                              ; preds = %323
  call void @llvm.lifetime.end.p0(ptr nonnull %28), !noalias !23464
  call void @llvm.lifetime.start.p0(ptr nonnull %27)
  %327 = add i64 %238, -1
  %328 = icmp ugt i64 %327, 4
  %329 = load ptr, ptr %217, align 8, !noalias !23464
  %330 = load i64, ptr %218, align 8, !noalias !23464
  %331 = add i64 %330, -1
  %332 = select i1 %328, i64 %331, i64 %327
  %333 = select i1 %328, ptr %329, ptr %217
  %334 = load ptr, ptr %52, align 8, !noalias !23464, !nonnull !1740, !noundef !1740
  %335 = getelementptr inbounds nuw i8, ptr %334, i64 16
  call void @llvm.lifetime.start.p0(ptr nonnull %9), !noalias !23493
; invoke <purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::term::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::term::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef align 16 captures(none) dereferenceable(96) %9, ptr noalias nofree noundef nonnull align 8 dereferenceable(216) %50, ptr noalias nofree noundef nonnull readonly align 4 captures(address, read_provenance) %333, i64 noundef range(i64 0, 1152921504606846976) %332, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(56) %335, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %4)
          to label %336 unwind label %545, !inline_history !23500

336:                                              ; preds = %326
  %337 = load i64, ptr %9, align 16, !range !2527, !noalias !23493, !noundef !1740
  %338 = icmp eq i64 %337, -1
  %339 = load i32, ptr %220, align 8, !noalias !23493
  %340 = load i32, ptr %221, align 4, !noalias !23493
  br i1 %338, label %346, label %341

341:                                              ; preds = %336
  %342 = getelementptr inbounds nuw i8, ptr %9, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(80) %27, ptr noundef nonnull align 16 dereferenceable(80) %342, i64 80, i1 false), !noalias !23501
  call void @llvm.lifetime.end.p0(ptr nonnull %9), !noalias !23493
  %343 = trunc i32 %339 to i8
  %344 = lshr i32 %339, 8
  %345 = trunc nuw i32 %344 to i24
  br label %366

346:                                              ; preds = %336
  call void @llvm.lifetime.end.p0(ptr nonnull %9), !noalias !23493
  %347 = icmp eq i32 %339, 2
  br i1 %347, label %348, label %349

348:                                              ; preds = %346
  call void @llvm.lifetime.end.p0(ptr nonnull %27)
  br label %383

349:                                              ; preds = %346
  call void @llvm.lifetime.start.p0(ptr nonnull %8), !noalias !23493
; invoke purrdf_sparql_eval::expr::ebv_term::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @purrdf_sparql_eval::expr::ebv_term::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef align 16 captures(none) dereferenceable(96) %8, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %4, i32 noundef %339, i32 noundef %340)
          to label %350 unwind label %545, !inline_history !23500

350:                                              ; preds = %349
  %351 = load i64, ptr %8, align 16, !range !2527, !noalias !23493, !noundef !1740
  %352 = icmp eq i64 %351, -1
  %353 = load i8, ptr %222, align 8, !noalias !23493
  br i1 %352, label %380, label %354

354:                                              ; preds = %350
  %355 = getelementptr inbounds nuw i8, ptr %8, i64 9
  %356 = load i24, ptr %355, align 1, !noalias !23501
  %357 = getelementptr inbounds nuw i8, ptr %8, i64 12
  %358 = load i32, ptr %357, align 4, !noalias !23501
  %359 = getelementptr inbounds nuw i8, ptr %8, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(80) %27, ptr noundef nonnull align 16 dereferenceable(80) %359, i64 80, i1 false), !noalias !23501
  call void @llvm.lifetime.end.p0(ptr nonnull %8), !noalias !23493
  br label %366

360:                                              ; preds = %323
  store ptr %237, ptr %212, align 8
  call void @llvm.lifetime.end.p0(ptr nonnull %28), !noalias !23464
  %361 = icmp ugt i64 %238, 5
  br i1 %361, label %362, label %536

362:                                              ; preds = %360
  %363 = load ptr, ptr %217, align 8, !nonnull !1740, !noundef !1740
  %364 = shl i64 %238, 3
  %365 = add i64 %364, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %363, i64 noundef %365, i64 noundef range(i64 1, -9223372036854775807) 4) #92, !noalias !23502
  br label %536

366:                                              ; preds = %354, %341
  %367 = phi i24 [ %345, %341 ], [ %356, %354 ]
  %368 = phi i8 [ %343, %341 ], [ %353, %354 ]
  %369 = phi i32 [ %340, %341 ], [ %358, %354 ]
  %370 = phi i64 [ %337, %341 ], [ %351, %354 ]
  %371 = getelementptr inbounds nuw i8, ptr %0, i64 25
  store i24 %367, ptr %371, align 1, !noalias !23505
  %372 = getelementptr inbounds nuw i8, ptr %0, i64 28
  store i32 %369, ptr %372, align 4, !noalias !23505
  %373 = getelementptr inbounds nuw i8, ptr %0, i64 32
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(80) %373, ptr noundef nonnull align 16 dereferenceable(80) %27, i64 80, i1 false), !noalias !23505
  call void @llvm.lifetime.end.p0(ptr nonnull %27)
  %374 = getelementptr inbounds nuw i8, ptr %0, i64 16
  store i64 %370, ptr %374, align 16, !alias.scope !23457, !noalias !23505
  %375 = getelementptr inbounds nuw i8, ptr %0, i64 24
  store i8 %368, ptr %375, align 8, !alias.scope !23457, !noalias !23505
  store i64 1, ptr %0, align 16, !alias.scope !23457, !noalias !23505
  %376 = icmp ugt i64 %238, 5
  br i1 %376, label %377, label %436

377:                                              ; preds = %366
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %329) ]
  %378 = shl i64 %238, 3
  %379 = add i64 %378, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %329, i64 noundef %379, i64 noundef range(i64 1, -9223372036854775807) 4) #92, !noalias !23506
  br label %436

380:                                              ; preds = %350
  call void @llvm.lifetime.end.p0(ptr nonnull %8), !noalias !23493
  call void @llvm.lifetime.end.p0(ptr nonnull %27)
  %381 = and i8 %353, 1
  %382 = icmp eq i8 %381, 0
  br i1 %382, label %383, label %415

383:                                              ; preds = %380, %348
  %384 = icmp ugt i64 %238, 5
  br i1 %384, label %385, label %425

385:                                              ; preds = %383
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %329) ]
  %386 = shl i64 %238, 3
  %387 = add i64 %386, -8
  %388 = load i64, ptr %223, align 8, !noalias !23509, !noundef !1740
  %389 = call i64 @llvm.umin.i64(i64 %387, i64 9223372036854775807)
  %390 = call i64 @llvm.ssub.sat.i64(i64 %388, i64 %389)
  store i64 %390, ptr %223, align 8, !noalias !23509
  %391 = load i64, ptr %224, align 8, !noalias !23509, !noundef !1740
  %392 = icmp slt i64 %390, %391
  br i1 %392, label %393, label %.preheader517

393:                                              ; preds = %385
  store i64 %390, ptr %224, align 8, !noalias !23509
  br label %.preheader517

.preheader517:                                    ; preds = %393, %385
  br label %394

394:                                              ; preds = %.preheader517, %397
  %395 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !23509
  %396 = icmp slt i64 %395, 0
  br i1 %396, label %397, label %__rustc::__rust_dealloc (.exit61)

397:                                              ; preds = %394
  %398 = add nsw i64 %395, 1
  %399 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %395, i64 %398 acq_rel acquire, align 8, !noalias !23509
  %400 = extractvalue { i64, i1 } %399, 1
  br i1 %400, label %401, label %394

401:                                              ; preds = %397
  %402 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %389 monotonic, align 8, !noalias !23509
  %403 = call i64 @llvm.ssub.sat.i64(i64 %402, i64 %389)
  %404 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !23509
  br label %405

405:                                              ; preds = %408, %401
  %406 = phi i64 [ %404, %401 ], [ %411, %408 ]
  %407 = icmp slt i64 %403, %406
  br i1 %407, label %408, label %412

408:                                              ; preds = %405
  %409 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %406, i64 %403 monotonic monotonic, align 8, !noalias !23509
  %410 = extractvalue { i64, i1 } %409, 1
  %411 = extractvalue { i64, i1 } %409, 0
  br i1 %410, label %412, label %405

412:                                              ; preds = %408, %405
  %413 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !23509
  br label %__rustc::__rust_dealloc (.exit61)

__rustc::__rust_dealloc (.exit61): ; preds = %394, %412
  %414 = icmp ne i64 %387, 0
  call void @llvm.assume(i1 %414), !noalias !23509
  call void @free(ptr noundef nonnull %329) #92, !noalias !23509
  br label %425

415:                                              ; preds = %380
  call void @llvm.lifetime.start.p0(ptr nonnull %26)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %26, ptr noundef nonnull align 8 dereferenceable(24) %219, i64 24, i1 false)
  call void @llvm.experimental.noalias.scope.decl(metadata !23512)
  %416 = load i64, ptr %32, align 8, !range !1835, !alias.scope !23512, !noalias !23515, !noundef !1740
  %417 = icmp eq i64 %234, %416
  br i1 %417, label %418, label %430

418:                                              ; preds = %415
; invoke <alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one
  invoke void @<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %32)
          to label %419 unwind label %421, !noalias !23515

419:                                              ; preds = %418
  %420 = load ptr, ptr %205, align 8, !alias.scope !23512, !noalias !23515
  br label %430

421:                                              ; preds = %418
  %422 = landingpad { ptr, i32 }
          cleanup
  store ptr %237, ptr %212, align 8
  %423 = icmp ugt i64 %238, 5
  br i1 %423, label %424, label %230

424:                                              ; preds = %421
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %329) ]
  br label %225

425:                                              ; preds = %430, %__rustc::__rust_dealloc (.exit61), %383
  %426 = phi ptr [ %233, %__rustc::__rust_dealloc (.exit61) ], [ %233, %383 ], [ %431, %430 ]
  %427 = phi i64 [ %234, %__rustc::__rust_dealloc (.exit61) ], [ %234, %383 ], [ %435, %430 ]
  %428 = phi ptr [ %235, %__rustc::__rust_dealloc (.exit61) ], [ %235, %383 ], [ %431, %430 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %29), !noalias !23464
  call void @llvm.lifetime.end.p0(ptr nonnull %30)
  call void @llvm.lifetime.start.p0(ptr nonnull %30)
  %429 = icmp eq ptr %237, %211
  br i1 %429, label %.loopexit88, label %232

430:                                              ; preds = %419, %415
  %431 = phi ptr [ %420, %419 ], [ %233, %415 ]
  %432 = getelementptr inbounds nuw [40 x i8], ptr %431, i64 %234
  store i64 %238, ptr %432, align 8, !noalias !23517
  %433 = getelementptr inbounds nuw i8, ptr %432, i64 8
  store ptr %329, ptr %433, align 8, !noalias !23517
  %434 = getelementptr inbounds nuw i8, ptr %432, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %434, ptr noundef nonnull align 8 dereferenceable(24) %26, i64 24, i1 false), !noalias !23517
  %435 = add i64 %234, 1
  store i64 %435, ptr %206, align 8, !alias.scope !23512, !noalias !23515
  call void @llvm.lifetime.end.p0(ptr nonnull %26)
  br label %425

436:                                              ; preds = %377, %366
  call void @llvm.lifetime.end.p0(ptr nonnull %29), !noalias !23464
  call void @llvm.lifetime.end.p0(ptr nonnull %30)
  %437 = ptrtoint ptr %211 to i64
  %438 = ptrtoint ptr %237 to i64
  %439 = sub nuw i64 %437, %438
  %440 = udiv exact i64 %439, 40
  call void @llvm.experimental.noalias.scope.decl(metadata !23518), !noalias !23469
  %441 = icmp eq ptr %211, %237
  br i1 %441, label %.loopexit87, label %.preheader86

.preheader86:                                     ; preds = %436, %479
  %442 = phi i64 [ %444, %479 ], [ 0, %436 ]
  %443 = getelementptr inbounds nuw [40 x i8], ptr %237, i64 %442
  %444 = add nuw nsw i64 %442, 1
  %445 = load i64, ptr %443, align 8, !range !1778, !alias.scope !23521, !noalias !23524, !noundef !1740
  %446 = icmp ugt i64 %445, 5
  br i1 %446, label %447, label %479

447:                                              ; preds = %.preheader86
  %448 = getelementptr i8, ptr %443, i64 8
  %449 = load ptr, ptr %448, align 8, !alias.scope !23518, !noalias !23524, !nonnull !1740, !noundef !1740
  %450 = shl i64 %445, 3
  %451 = add i64 %450, -8
  %452 = load i64, ptr %223, align 8, !noalias !23529, !noundef !1740
  %453 = call i64 @llvm.umin.i64(i64 %451, i64 9223372036854775807)
  %454 = call i64 @llvm.ssub.sat.i64(i64 %452, i64 %453)
  store i64 %454, ptr %223, align 8, !noalias !23529
  %455 = load i64, ptr %224, align 8, !noalias !23529, !noundef !1740
  %456 = icmp slt i64 %454, %455
  br i1 %456, label %457, label %.preheader516

457:                                              ; preds = %447
  store i64 %454, ptr %224, align 8, !noalias !23529
  br label %.preheader516

.preheader516:                                    ; preds = %457, %447
  br label %458

458:                                              ; preds = %.preheader516, %461
  %459 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !23529
  %460 = icmp slt i64 %459, 0
  br i1 %460, label %461, label %__rustc::__rust_dealloc (.exit62)

461:                                              ; preds = %458
  %462 = add nsw i64 %459, 1
  %463 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %459, i64 %462 acq_rel acquire, align 8, !noalias !23529
  %464 = extractvalue { i64, i1 } %463, 1
  br i1 %464, label %465, label %458

465:                                              ; preds = %461
  %466 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %453 monotonic, align 8, !noalias !23529
  %467 = call i64 @llvm.ssub.sat.i64(i64 %466, i64 %453)
  %468 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !23529
  br label %469

469:                                              ; preds = %472, %465
  %470 = phi i64 [ %468, %465 ], [ %475, %472 ]
  %471 = icmp slt i64 %467, %470
  br i1 %471, label %472, label %476

472:                                              ; preds = %469
  %473 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %470, i64 %467 monotonic monotonic, align 8, !noalias !23529
  %474 = extractvalue { i64, i1 } %473, 1
  %475 = extractvalue { i64, i1 } %473, 0
  br i1 %474, label %476, label %469

476:                                              ; preds = %472, %469
  %477 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !23529
  br label %__rustc::__rust_dealloc (.exit62)

__rustc::__rust_dealloc (.exit62): ; preds = %458, %476
  %478 = icmp ne i64 %451, 0
  call void @llvm.assume(i1 %478), !noalias !23529
  call void @free(ptr noundef nonnull %449) #92, !noalias !23529
  br label %479

479:                                              ; preds = %__rustc::__rust_dealloc (.exit62), %.preheader86
  %480 = icmp eq i64 %444, %440
  br i1 %480, label %.loopexit87, label %.preheader86

.loopexit87:                                      ; preds = %479, %436
  %481 = icmp eq i64 %209, 0
  br i1 %481, label %484, label %482

482:                                              ; preds = %.loopexit87
  %483 = mul nuw i64 %209, 40
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %208, i64 noundef %483, i64 noundef range(i64 1, -9223372036854775807) 8) #92, !noalias !23524
  br label %484

484:                                              ; preds = %482, %.loopexit87
  call void @llvm.lifetime.end.p0(ptr nonnull %31), !noalias !23464
  call void @llvm.experimental.noalias.scope.decl(metadata !23532)
  call void @llvm.experimental.noalias.scope.decl(metadata !23535), !noalias !23469
  %485 = icmp eq i64 %234, 0
  br i1 %485, label %.loopexit85, label %.preheader84

.preheader84:                                     ; preds = %484, %523
  %486 = phi i64 [ %488, %523 ], [ 0, %484 ]
  %487 = getelementptr inbounds nuw [40 x i8], ptr %235, i64 %486
  %488 = add nuw nsw i64 %486, 1
  %489 = load i64, ptr %487, align 8, !range !1778, !alias.scope !23538, !noalias !23541, !noundef !1740
  %490 = icmp ugt i64 %489, 5
  br i1 %490, label %491, label %523

491:                                              ; preds = %.preheader84
  %492 = getelementptr i8, ptr %487, i64 8
  %493 = load ptr, ptr %492, align 8, !alias.scope !23535, !noalias !23541, !nonnull !1740, !noundef !1740
  %494 = shl i64 %489, 3
  %495 = add i64 %494, -8
  %496 = load i64, ptr %223, align 8, !noalias !23542, !noundef !1740
  %497 = call i64 @llvm.umin.i64(i64 %495, i64 9223372036854775807)
  %498 = call i64 @llvm.ssub.sat.i64(i64 %496, i64 %497)
  store i64 %498, ptr %223, align 8, !noalias !23542
  %499 = load i64, ptr %224, align 8, !noalias !23542, !noundef !1740
  %500 = icmp slt i64 %498, %499
  br i1 %500, label %501, label %.preheader515

501:                                              ; preds = %491
  store i64 %498, ptr %224, align 8, !noalias !23542
  br label %.preheader515

.preheader515:                                    ; preds = %501, %491
  br label %502

502:                                              ; preds = %.preheader515, %505
  %503 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !23542
  %504 = icmp slt i64 %503, 0
  br i1 %504, label %505, label %__rustc::__rust_dealloc (.exit63)

505:                                              ; preds = %502
  %506 = add nsw i64 %503, 1
  %507 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %503, i64 %506 acq_rel acquire, align 8, !noalias !23542
  %508 = extractvalue { i64, i1 } %507, 1
  br i1 %508, label %509, label %502

509:                                              ; preds = %505
  %510 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %497 monotonic, align 8, !noalias !23542
  %511 = call i64 @llvm.ssub.sat.i64(i64 %510, i64 %497)
  %512 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !23542
  br label %513

513:                                              ; preds = %516, %509
  %514 = phi i64 [ %512, %509 ], [ %519, %516 ]
  %515 = icmp slt i64 %511, %514
  br i1 %515, label %516, label %520

516:                                              ; preds = %513
  %517 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %514, i64 %511 monotonic monotonic, align 8, !noalias !23542
  %518 = extractvalue { i64, i1 } %517, 1
  %519 = extractvalue { i64, i1 } %517, 0
  br i1 %518, label %520, label %513

520:                                              ; preds = %516, %513
  %521 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !23542
  br label %__rustc::__rust_dealloc (.exit63)

__rustc::__rust_dealloc (.exit63): ; preds = %502, %520
  %522 = icmp ne i64 %495, 0
  call void @llvm.assume(i1 %522), !noalias !23542
  call void @free(ptr noundef nonnull %493) #92, !noalias !23542
  br label %523

523:                                              ; preds = %__rustc::__rust_dealloc (.exit63), %.preheader84
  %524 = icmp eq i64 %488, %234
  br i1 %524, label %.loopexit85, label %.preheader84

.loopexit85:                                      ; preds = %523, %484
  %525 = load i64, ptr %32, align 8, !alias.scope !23532, !noalias !23469
  %526 = icmp eq i64 %525, 0
  br i1 %526, label %535, label %527

527:                                              ; preds = %.loopexit85
  %528 = mul nuw i64 %525, 40
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %235, i64 noundef %528, i64 noundef range(i64 1, -9223372036854775807) 8) #92, !noalias !23541
  br label %535

529:                                              ; preds = %1353, %1139, %578, %575, %571, %533, %230
  %530 = phi i8 [ 1, %533 ], [ 0, %230 ], [ %539, %1353 ], [ %539, %1139 ], [ 1, %578 ], [ 1, %571 ], [ 1, %575 ]
  %531 = phi i1 [ true, %533 ], [ true, %230 ], [ true, %1353 ], [ false, %1139 ], [ true, %578 ], [ true, %571 ], [ true, %575 ]
  %532 = phi { ptr, i32 } [ %534, %533 ], [ %231, %230 ], [ %1354, %1353 ], [ %1140, %1139 ], [ %572, %578 ], [ %572, %571 ], [ %572, %575 ]
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>(ptr noalias nofree noundef align 8 dereferenceable(216) %50) #89
          to label %192 unwind label %550, !noalias !23469

533:                                              ; preds = %1361, %851, %552
  %534 = landingpad { ptr, i32 }
          cleanup
  br label %529

535:                                              ; preds = %527, %.loopexit85
  call void @llvm.lifetime.end.p0(ptr nonnull %32), !noalias !23464
  br label %1289

536:                                              ; preds = %362, %360
  call void @llvm.lifetime.end.p0(ptr nonnull %29), !noalias !23464
  br label %243

537:                                              ; preds = %__rustc::__rust_dealloc (.exit60), %.loopexit83
  call void @llvm.lifetime.end.p0(ptr nonnull %31), !noalias !23464
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %49, ptr noundef nonnull align 8 dereferenceable(24) %32, i64 24, i1 false), !noalias !23464
  call void @llvm.lifetime.end.p0(ptr nonnull %32), !noalias !23464
  br label %538

538:                                              ; preds = %1138, %537
  %539 = phi i8 [ 1, %1138 ], [ 0, %537 ]
  %540 = getelementptr inbounds nuw i8, ptr %4, i64 696
  call void @llvm.experimental.noalias.scope.decl(metadata !23545)
  %541 = load ptr, ptr %540, align 8, !alias.scope !23545, !noalias !23548, !nonnull !1740, !noundef !1740
  %542 = getelementptr inbounds nuw i8, ptr %541, i64 40
  %543 = load atomic i32, ptr %542 acquire, align 4, !noalias !23550
  %544 = icmp eq i32 %543, 0
  br i1 %544, label %1141, label %1151

545:                                              ; preds = %349, %326, %241
  %546 = landingpad { ptr, i32 }
          cleanup
  store ptr %237, ptr %212, align 8
  %547 = icmp ugt i64 %238, 5
  br i1 %547, label %548, label %230

548:                                              ; preds = %545
  %549 = load ptr, ptr %217, align 8, !nonnull !1740, !noundef !1740
  br label %225

550:                                              ; preds = %1531, %1530, %1462, %1362, %915, %578, %529, %192
  %551 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #90, !noalias !23457
  unreachable

552:                                              ; preds = %203
  %553 = getelementptr inbounds nuw i8, ptr %51, i64 184
  %554 = load i64, ptr %553, align 8, !noundef !1740
  %555 = tail call noundef range(i64 0, 230584300921369396) i64 @llvm.umin.i64(i64 %554, i64 range(i64 0, 230584300921369396) %188)
  %556 = getelementptr inbounds nuw i8, ptr %57, i64 8
  %557 = load ptr, ptr %556, align 8, !alias.scope !23460, !noalias !23468, !nonnull !1740, !noundef !1740
  call void @llvm.lifetime.start.p0(ptr nonnull %48), !noalias !23464
; invoke <purrdf_sparql_eval::eval::EvalCtx>::loop_snapshot
  %558 = invoke fastcc noundef ptr @<purrdf_sparql_eval::eval::EvalCtx>::loop_snapshot(ptr noundef nonnull align 16 dereferenceable(1248) %4, i64 noundef %555)
          to label %559 unwind label %533, !noalias !23469

559:                                              ; preds = %552
  store ptr %558, ptr %48, align 8, !noalias !23464
  call void @llvm.lifetime.start.p0(ptr nonnull %47)
  call void @llvm.lifetime.start.p0(ptr nonnull %46), !noalias !23464
  %560 = getelementptr inbounds nuw i8, ptr %4, i64 616
  %561 = load ptr, ptr %560, align 8, !noundef !1740
  %562 = icmp eq ptr %561, null
  br i1 %562, label %581, label %563

563:                                              ; preds = %559
  %564 = getelementptr inbounds nuw i8, ptr %561, i64 16
  %565 = load i64, ptr %564, align 8
  %566 = icmp ugt i64 %565, -3
  br i1 %566, label %567, label %581

567:                                              ; preds = %563
  %568 = getelementptr inbounds nuw i8, ptr %561, i64 40
  %569 = load i64, ptr %568, align 8
  %570 = icmp ult i64 %569, -2
  br label %581

571:                                              ; preds = %1364, %1362, %838, %624, %579
  %572 = phi { ptr, i32 } [ %1365, %1364 ], [ %625, %624 ], [ %580, %579 ], [ %1363, %1362 ], [ %839, %838 ]
  call void @llvm.experimental.noalias.scope.decl(metadata !23551)
  %573 = load ptr, ptr %48, align 8, !alias.scope !23551, !noalias !23469, !noundef !1740
  %574 = icmp eq ptr %573, null
  br i1 %574, label %529, label %575

575:                                              ; preds = %571
  %576 = atomicrmw sub ptr %573, i64 1 release, align 8, !noalias !23554
  %577 = icmp eq i64 %576, 1
  br i1 %577, label %578, label %529

578:                                              ; preds = %575
  fence acquire, !noalias !23469
; invoke <alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %48) #91
          to label %529 unwind label %550, !inline_history !2025

579:                                              ; preds = %592, %591
  %580 = landingpad { ptr, i32 }
          cleanup
  br label %571

581:                                              ; preds = %567, %563, %559
  %582 = phi i1 [ false, %559 ], [ true, %563 ], [ %570, %567 ]
  %583 = getelementptr inbounds nuw i8, ptr %4, i64 1234
  %584 = load i8, ptr %583, align 2, !range !1747, !noundef !1740
  %585 = trunc nuw i8 %584 to i1
  call void @llvm.lifetime.start.p0(ptr nonnull %45), !noalias !23464
  store ptr %4, ptr %45, align 8, !noalias !23464
  %586 = getelementptr inbounds nuw i8, ptr %45, i64 8
  store ptr %48, ptr %586, align 8, !noalias !23464
  %587 = getelementptr inbounds nuw i8, ptr %45, i64 16
  store ptr %51, ptr %587, align 8, !noalias !23464
  %588 = getelementptr inbounds nuw i8, ptr %45, i64 24
  store ptr %50, ptr %588, align 8, !noalias !23464
  call void @llvm.lifetime.start.p0(ptr nonnull %44), !noalias !23464
  store ptr %557, ptr %44, align 8, !noalias !23464
  %589 = getelementptr inbounds nuw i8, ptr %44, i64 8
  store i64 %555, ptr %589, align 8, !noalias !23464
  %590 = getelementptr inbounds nuw i8, ptr %44, i64 16
  store ptr %52, ptr %590, align 8, !noalias !23464
  br i1 %582, label %592, label %591

591:                                              ; preds = %581
; invoke purrdf_sparql_eval::parallel::par_chunk_try_map_init::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_filter_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#0}, purrdf_sparql_eval::expr::eval_filter_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#1}, purrdf_sparql_eval::expr::eval_filter_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#2}>
  invoke fastcc void @purrdf_sparql_eval::parallel::par_chunk_try_map_init::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_filter_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#0}, purrdf_sparql_eval::expr::eval_filter_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#1}, purrdf_sparql_eval::expr::eval_filter_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#2}>(ptr noalias nofree noundef nonnull align 16 captures(address) dereferenceable(256) %46, i1 noundef zeroext %585, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) %557, i64 noundef range(i64 0, 230584300921369396) %555, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(32) %45, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(24) %44, ptr noundef nonnull align 8 %51)
          to label %593 unwind label %579, !inline_history !23557

592:                                              ; preds = %581
; invoke purrdf_sparql_eval::parallel::par_blocks_try_map_init::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_filter_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#0}, purrdf_sparql_eval::expr::eval_filter_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#1}, purrdf_sparql_eval::expr::eval_filter_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#2}>
  invoke fastcc void @purrdf_sparql_eval::parallel::par_blocks_try_map_init::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_filter_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#0}, purrdf_sparql_eval::expr::eval_filter_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#1}, purrdf_sparql_eval::expr::eval_filter_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#2}>(ptr noalias nofree noundef nonnull align 16 captures(address) dereferenceable(256) %46, i1 noundef zeroext %585, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) %557, i64 noundef range(i64 0, 230584300921369396) %555, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(32) %45, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(24) %44, ptr noundef nonnull align 8 %51)
          to label %593 unwind label %579, !inline_history !23557

593:                                              ; preds = %592, %591
  call void @llvm.lifetime.end.p0(ptr nonnull %44), !noalias !23464
  call void @llvm.lifetime.end.p0(ptr nonnull %45), !noalias !23464
  %594 = load i64, ptr %46, align 16, !range !2059, !noalias !23464, !noundef !1740
  %595 = icmp eq i64 %594, -1
  br i1 %595, label %596, label %602

596:                                              ; preds = %593
  %597 = getelementptr inbounds nuw i8, ptr %46, i64 16
  %598 = getelementptr inbounds nuw i8, ptr %46, i64 48
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(64) %47, ptr noundef nonnull align 16 dereferenceable(64) %598, i64 64, i1 false), !noalias !23464
  %599 = getelementptr inbounds nuw i8, ptr %0, i64 48
  %600 = getelementptr inbounds nuw i8, ptr %0, i64 16
  %601 = load <4 x i64>, ptr %597, align 16, !noalias !23464
  call void @llvm.lifetime.end.p0(ptr nonnull %46), !noalias !23464
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(64) %599, ptr noundef nonnull align 16 dereferenceable(64) %47, i64 64, i1 false), !noalias !23505
  store <4 x i64> %601, ptr %600, align 16, !alias.scope !23457, !noalias !23505
  store i64 1, ptr %0, align 16, !alias.scope !23457, !noalias !23505
  call void @llvm.lifetime.end.p0(ptr nonnull %47)
  br label %1355

602:                                              ; preds = %593
  %603 = getelementptr inbounds nuw i8, ptr %46, i64 8
  %604 = getelementptr inbounds nuw i8, ptr %46, i64 24
  %605 = load i64, ptr %604, align 8, !noalias !23464
  %606 = getelementptr inbounds nuw i8, ptr %46, i64 32
  %607 = load i64, ptr %606, align 16, !noalias !23464
  %608 = getelementptr inbounds nuw i8, ptr %46, i64 40
  %609 = load i64, ptr %608, align 8, !noalias !23464
  %610 = getelementptr inbounds nuw i8, ptr %46, i64 48
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(208) %47, ptr noundef nonnull align 16 dereferenceable(208) %610, i64 208, i1 false), !noalias !23464
  %611 = getelementptr inbounds nuw i8, ptr %40, i64 24
  %612 = getelementptr inbounds nuw i8, ptr %14, i64 8
  %613 = load <2 x i64>, ptr %603, align 8, !noalias !23464
  call void @llvm.lifetime.end.p0(ptr nonnull %46), !noalias !23464
  call void @llvm.lifetime.start.p0(ptr nonnull %40), !noalias !23464
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(208) %611, ptr noundef nonnull align 16 dereferenceable(208) %47, i64 208, i1 false), !noalias !23464
  store i64 %594, ptr %14, align 8
  store <2 x i64> %613, ptr %612, align 8
  call void @llvm.lifetime.end.p0(ptr nonnull %47)
  call void @llvm.lifetime.start.p0(ptr nonnull %41), !noalias !23464
  %614 = add i64 %605, -3
  %615 = icmp ult i64 %614, -2
  %616 = select i1 %615, i64 %609, i64 %605
  %617 = add i64 %616, -1
  %618 = select i1 %615, i64 %605, i64 1
  %619 = select i1 %615, i64 1, i64 %609
  store i64 %618, ptr %40, align 8, !noalias !23464
  %620 = getelementptr inbounds nuw i8, ptr %40, i64 8
  store i64 %607, ptr %620, align 8, !noalias !23464
  %621 = getelementptr inbounds nuw i8, ptr %40, i64 16
  store i64 %619, ptr %621, align 8, !noalias !23464
  %622 = getelementptr inbounds nuw i8, ptr %40, i64 232
  store i64 0, ptr %622, align 8, !noalias !23464
  %623 = getelementptr inbounds nuw i8, ptr %40, i64 240
  store i64 %617, ptr %623, align 8, !noalias !23464
; invoke <purrdf_core::small::IntoIter<[(purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint); 1]> as core::iter::traits::iterator::Iterator>::unzip::<purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>
  invoke fastcc void @<purrdf_core::small::IntoIter<[(purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint); 1]> as core::iter::traits::iterator::Iterator>::unzip::<purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>(ptr noalias nofree noundef align 8 captures(none) dereferenceable(240) %41, ptr noalias nofree noundef align 8 captures(address) dereferenceable(248) %40)
          to label %626 unwind label %1364, !noalias !23469

624:                                              ; preds = %796
  %625 = landingpad { ptr, i32 }
          cleanup
  br label %571

626:                                              ; preds = %602
  call void @llvm.lifetime.end.p0(ptr nonnull %40), !noalias !23464
  call void @llvm.lifetime.start.p0(ptr nonnull %43), !noalias !23464
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %43, ptr noundef nonnull align 8 dereferenceable(32) %41, i64 32, i1 false), !noalias !23464
  call void @llvm.lifetime.start.p0(ptr nonnull %42), !noalias !23464
  %627 = getelementptr inbounds nuw i8, ptr %41, i64 32
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(208) %42, ptr noundef nonnull align 8 dereferenceable(208) %627, i64 208, i1 false), !noalias !23464
  call void @llvm.lifetime.end.p0(ptr nonnull %41), !noalias !23464
  call void @llvm.lifetime.start.p0(ptr nonnull %38)
  call void @llvm.lifetime.start.p0(ptr nonnull %37)
  call void @llvm.experimental.noalias.scope.decl(metadata !23558)
  call void @llvm.experimental.noalias.scope.decl(metadata !23561)
  %628 = getelementptr inbounds nuw i8, ptr %51, i64 194
  %629 = load i8, ptr %628, align 2, !range !3730, !alias.scope !23558, !noalias !23563, !noundef !1740
  %630 = icmp ne i8 %629, 2
  %631 = load ptr, ptr %560, align 8, !alias.scope !23567, !noalias !23568
  %632 = icmp eq ptr %631, null
  %633 = select i1 %630, i1 true, i1 %632
  br i1 %633, label %634, label %637

634:                                              ; preds = %626
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %37, ptr noundef nonnull align 8 dereferenceable(24) %14, i64 24, i1 false)
; invoke core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>(ptr noalias nofree noundef nonnull align 8 dereferenceable(208) %42)
          to label %803 unwind label %635, !noalias !23469

635:                                              ; preds = %634
  %636 = landingpad { ptr, i32 }
          cleanup
  br label %1362

637:                                              ; preds = %626
  %638 = load i64, ptr %42, align 8, !noalias !23569
  %639 = getelementptr inbounds nuw i8, ptr %42, i64 8
  %640 = load i64, ptr %639, align 8, !noalias !23569
  %641 = getelementptr inbounds nuw i8, ptr %42, i64 16
  %642 = load i64, ptr %641, align 8, !noalias !23569
  %643 = getelementptr inbounds nuw i8, ptr %42, i64 24
  %644 = getelementptr inbounds nuw i8, ptr %19, i64 24
  call void @llvm.lifetime.start.p0(ptr nonnull %19), !noalias !23570
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(184) %644, ptr noundef nonnull align 8 dereferenceable(184) %643, i64 184, i1 false), !noalias !23469
  call void @llvm.experimental.noalias.scope.decl(metadata !23577)
  call void @llvm.lifetime.start.p0(ptr nonnull %20), !noalias !23570
  call void @llvm.experimental.noalias.scope.decl(metadata !23578)
  %645 = icmp ugt i64 %638, 2
  %646 = select i1 %645, i64 %642, i64 %638
  %647 = add i64 %646, -1
  %648 = select i1 %645, i64 %638, i64 1
  %649 = select i1 %645, i64 1, i64 %642
  store i64 %648, ptr %19, align 8, !alias.scope !23581, !noalias !23583
  %650 = getelementptr inbounds nuw i8, ptr %19, i64 8
  store i64 %640, ptr %650, align 8, !alias.scope !23581, !noalias !23583
  %651 = getelementptr inbounds nuw i8, ptr %19, i64 16
  store i64 %649, ptr %651, align 8, !alias.scope !23581, !noalias !23583
  %652 = getelementptr inbounds nuw i8, ptr %19, i64 208
  store i64 0, ptr %652, align 8, !alias.scope !23584, !noalias !23585
  %653 = getelementptr inbounds nuw i8, ptr %19, i64 216
  store i64 %647, ptr %653, align 8, !alias.scope !23584, !noalias !23585
; invoke <purrdf_core::small::IntoIter<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]> as core::iter::traits::iterator::Iterator>::collect::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>
  invoke fastcc void @<purrdf_core::small::IntoIter<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]> as core::iter::traits::iterator::Iterator>::collect::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>(ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %20, ptr noalias nofree noundef align 8 captures(address) dereferenceable(224) %19)
          to label %656 unwind label %654, !noalias !23583

654:                                              ; preds = %637
  %655 = landingpad { ptr, i32 }
          cleanup
  br label %794

656:                                              ; preds = %637
  call void @llvm.lifetime.end.p0(ptr nonnull %19), !noalias !23570
  %657 = getelementptr inbounds nuw i8, ptr %20, i64 8
  %658 = load ptr, ptr %657, align 8, !noalias !23570, !nonnull !1740, !noundef !1740
  %659 = getelementptr inbounds nuw i8, ptr %20, i64 16
  %660 = load i64, ptr %659, align 8, !noalias !23570, !noundef !1740
  %661 = mul nuw nsw i64 %660, 200
  %662 = getelementptr inbounds nuw i8, ptr %658, i64 %661
  %663 = icmp eq i64 %660, 0
  br i1 %663, label %731, label %.preheader81.preheader

.preheader81.preheader:                           ; preds = %656
  %xtraiter = and i64 %660, 3
  %664 = icmp ult i64 %660, 4
  br i1 %664, label %.preheader81.epil.preheader, label %.preheader81.preheader.new

.preheader81.preheader.new:                       ; preds = %.preheader81.preheader
  %unroll_iter = and i64 %660, -4
  br label %.preheader81

.preheader81:                                     ; preds = %709, %.preheader81.preheader.new
  %665 = phi i64 [ 0, %.preheader81.preheader.new ], [ %712, %709 ]
  %666 = phi i64 [ 0, %.preheader81.preheader.new ], [ %711, %709 ]
  %niter = phi i64 [ 0, %.preheader81.preheader.new ], [ %niter.next.3, %709 ]
  %667 = getelementptr inbounds nuw [200 x i8], ptr %658, i64 %665
  %668 = getelementptr i8, ptr %667, i64 168
  %669 = load i64, ptr %668, align 8, !noalias !23583, !noundef !1740
  %670 = getelementptr i8, ptr %667, i64 176
  %671 = load i64, ptr %670, align 8, !noalias !23583, !noundef !1740
  %672 = call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %671, i64 %669)
  %673 = extractvalue { i64, i1 } %672, 0
  %674 = extractvalue { i64, i1 } %672, 1
  br i1 %674, label %675, label %.preheader81.1, !prof !1742

675:                                              ; preds = %.preheader81
  br label %.preheader81.1

.preheader81.1:                                   ; preds = %675, %.preheader81
  %676 = phi i64 [ -1, %675 ], [ %673, %.preheader81 ]
  %677 = call noundef i64 @llvm.uadd.sat.i64(i64 %666, i64 %676)
  %678 = getelementptr inbounds nuw [200 x i8], ptr %658, i64 %665
  %679 = getelementptr i8, ptr %678, i64 368
  %680 = load i64, ptr %679, align 8, !noalias !23583, !noundef !1740
  %681 = getelementptr i8, ptr %678, i64 376
  %682 = load i64, ptr %681, align 8, !noalias !23583, !noundef !1740
  %683 = call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %682, i64 %680)
  %684 = extractvalue { i64, i1 } %683, 0
  %685 = extractvalue { i64, i1 } %683, 1
  br i1 %685, label %686, label %.preheader81.2, !prof !1742

686:                                              ; preds = %.preheader81.1
  br label %.preheader81.2

.preheader81.2:                                   ; preds = %686, %.preheader81.1
  %687 = phi i64 [ -1, %686 ], [ %684, %.preheader81.1 ]
  %688 = call noundef i64 @llvm.uadd.sat.i64(i64 %677, i64 %687)
  %689 = getelementptr inbounds nuw [200 x i8], ptr %658, i64 %665
  %690 = getelementptr i8, ptr %689, i64 568
  %691 = load i64, ptr %690, align 8, !noalias !23583, !noundef !1740
  %692 = getelementptr i8, ptr %689, i64 576
  %693 = load i64, ptr %692, align 8, !noalias !23583, !noundef !1740
  %694 = call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %693, i64 %691)
  %695 = extractvalue { i64, i1 } %694, 0
  %696 = extractvalue { i64, i1 } %694, 1
  br i1 %696, label %697, label %.preheader81.3, !prof !1742

697:                                              ; preds = %.preheader81.2
  br label %.preheader81.3

.preheader81.3:                                   ; preds = %697, %.preheader81.2
  %698 = phi i64 [ -1, %697 ], [ %695, %.preheader81.2 ]
  %699 = call noundef i64 @llvm.uadd.sat.i64(i64 %688, i64 %698)
  %700 = getelementptr inbounds nuw [200 x i8], ptr %658, i64 %665
  %701 = getelementptr i8, ptr %700, i64 768
  %702 = load i64, ptr %701, align 8, !noalias !23583, !noundef !1740
  %703 = getelementptr i8, ptr %700, i64 776
  %704 = load i64, ptr %703, align 8, !noalias !23583, !noundef !1740
  %705 = call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %704, i64 %702)
  %706 = extractvalue { i64, i1 } %705, 0
  %707 = extractvalue { i64, i1 } %705, 1
  br i1 %707, label %708, label %709, !prof !1742

708:                                              ; preds = %.preheader81.3
  br label %709

709:                                              ; preds = %708, %.preheader81.3
  %710 = phi i64 [ -1, %708 ], [ %706, %.preheader81.3 ]
  %711 = call noundef i64 @llvm.uadd.sat.i64(i64 %699, i64 %710)
  %712 = add nuw i64 %665, 4
  %niter.next.3 = add i64 %niter, 4
  %niter.ncmp.3 = icmp eq i64 %niter.next.3, %unroll_iter
  br i1 %niter.ncmp.3, label %.unr-lcssa, label %.preheader81

713:                                              ; preds = %731
  %714 = landingpad { ptr, i32 }
          cleanup
  br label %794

.unr-lcssa:                                       ; preds = %709
  %lcmp.mod.not = icmp eq i64 %xtraiter, 0
  br i1 %lcmp.mod.not, label %.epilog-lcssa, label %.preheader81.epil.preheader

.preheader81.epil.preheader:                      ; preds = %.unr-lcssa, %.preheader81.preheader
  %.epil.init = phi i64 [ 0, %.preheader81.preheader ], [ %712, %.unr-lcssa ]
  %.epil.init560 = phi i64 [ 0, %.preheader81.preheader ], [ %711, %.unr-lcssa ]
  %lcmp.mod562 = icmp ne i64 %xtraiter, 0
  call void @llvm.assume(i1 %lcmp.mod562)
  br label %.preheader81.epil

.preheader81.epil:                                ; preds = %726, %.preheader81.epil.preheader
  %715 = phi i64 [ %729, %726 ], [ %.epil.init, %.preheader81.epil.preheader ]
  %716 = phi i64 [ %728, %726 ], [ %.epil.init560, %.preheader81.epil.preheader ]
  %epil.iter = phi i64 [ %epil.iter.next, %726 ], [ 0, %.preheader81.epil.preheader ]
  %717 = getelementptr inbounds nuw [200 x i8], ptr %658, i64 %715
  %718 = getelementptr i8, ptr %717, i64 168
  %719 = load i64, ptr %718, align 8, !noalias !23583, !noundef !1740
  %720 = getelementptr i8, ptr %717, i64 176
  %721 = load i64, ptr %720, align 8, !noalias !23583, !noundef !1740
  %722 = call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %721, i64 %719)
  %723 = extractvalue { i64, i1 } %722, 0
  %724 = extractvalue { i64, i1 } %722, 1
  br i1 %724, label %725, label %726, !prof !1742

725:                                              ; preds = %.preheader81.epil
  br label %726

726:                                              ; preds = %725, %.preheader81.epil
  %727 = phi i64 [ -1, %725 ], [ %723, %.preheader81.epil ]
  %728 = call noundef i64 @llvm.uadd.sat.i64(i64 %716, i64 %727)
  %729 = add nuw i64 %715, 1
  %epil.iter.next = add i64 %epil.iter, 1
  %epil.iter.cmp.not = icmp eq i64 %epil.iter.next, %xtraiter
  br i1 %epil.iter.cmp.not, label %.epilog-lcssa, label %.preheader81.epil, !llvm.loop !23586

.epilog-lcssa:                                    ; preds = %726, %.unr-lcssa
  %.lcssa512 = phi i64 [ %711, %.unr-lcssa ], [ %728, %726 ]
  %730 = icmp eq i64 %.lcssa512, 0
  br i1 %730, label %731, label %737

731:                                              ; preds = %741, %737, %.epilog-lcssa, %656
  call void @llvm.lifetime.start.p0(ptr nonnull %18), !noalias !23570
  %732 = load i64, ptr %20, align 8, !range !1835, !noalias !23570, !noundef !1740
  %733 = icmp ult i64 %660, 46116860184273880
  call void @llvm.assume(i1 %733)
  store ptr %658, ptr %18, align 8, !noalias !23570
  %734 = getelementptr inbounds nuw i8, ptr %18, i64 8
  store ptr %658, ptr %734, align 8, !noalias !23570
  %735 = getelementptr inbounds nuw i8, ptr %18, i64 16
  store i64 %732, ptr %735, align 8, !noalias !23570
  %736 = getelementptr inbounds nuw i8, ptr %18, i64 24
  store ptr %662, ptr %736, align 8, !noalias !23570
; invoke <purrdf_sparql_eval::row_checkpoint::Committing>::of::<core::iter::adapters::map::Map<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>, <purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::commit<purrdf_core::ir::dataset::RdfDataset, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>, <purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::commit_rows<purrdf_core::ir::dataset::RdfDataset, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>::{closure#0}>::{closure#1}>>
  invoke fastcc void @<purrdf_sparql_eval::row_checkpoint::Committing>::of::<core::iter::adapters::map::Map<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>, <purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::commit<purrdf_core::ir::dataset::RdfDataset, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>, <purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::commit_rows<purrdf_core::ir::dataset::RdfDataset, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>::{closure#0}>::{closure#1}>>(ptr noalias nofree noundef align 8 captures(none) dereferenceable(32) %15, ptr noalias nofree noundef align 8 captures(address) dereferenceable(32) %18)
          to label %744 unwind label %713

737:                                              ; preds = %.epilog-lcssa
  %738 = getelementptr inbounds nuw i8, ptr %631, i64 336
  %739 = load ptr, ptr %738, align 8, !noalias !23583, !noundef !1740
  %740 = icmp eq ptr %739, null
  br i1 %740, label %731, label %741

741:                                              ; preds = %737
  %742 = getelementptr inbounds nuw i8, ptr %631, i64 352
  %743 = atomicrmw add ptr %742, i64 %.lcssa512 monotonic, align 8, !noalias !23583
  br label %731

744:                                              ; preds = %731
  call void @llvm.lifetime.end.p0(ptr nonnull %18), !noalias !23570
  %745 = getelementptr inbounds nuw i8, ptr %51, i64 193
  %746 = load i8, ptr %745, align 1, !range !1747, !alias.scope !23577, !noalias !23587, !noundef !1740
  %747 = zext nneg i8 %746 to i64
  %748 = getelementptr inbounds nuw i8, ptr %15, i64 8
  %749 = load ptr, ptr %748, align 8, !nonnull !1740, !noundef !1740
  %750 = getelementptr inbounds nuw i8, ptr %15, i64 16
  %751 = load i64, ptr %750, align 8, !noundef !1740
  %752 = icmp eq i64 %751, 0
  br i1 %752, label %.loopexit80, label %iter.check

iter.check:                                       ; preds = %744
  %min.iters.check = icmp ult i64 %751, 8
  br i1 %min.iters.check, label %.preheader79.preheader, label %vector.main.loop.iter.check

vector.main.loop.iter.check:                      ; preds = %iter.check
  %min.iters.check390 = icmp ult i64 %751, 32
  br i1 %min.iters.check390, label %vec.epilog.ph, label %vector.ph

vector.ph:                                        ; preds = %vector.main.loop.iter.check
  %n.mod.vf = and i64 %751, 24
  %n.vec = and i64 %751, -32
  br label %vector.body

vector.body:                                      ; preds = %vector.body, %vector.ph
  %index = phi i64 [ 0, %vector.ph ], [ %index.next, %vector.body ]
  %vec.ind = phi <8 x i64> [ <i64 0, i64 1, i64 2, i64 3, i64 4, i64 5, i64 6, i64 7>, %vector.ph ], [ %vec.ind.next, %vector.body ]
  %vec.phi = phi <8 x i64> [ zeroinitializer, %vector.ph ], [ %753, %vector.body ]
  %vec.phi391 = phi <8 x i64> [ zeroinitializer, %vector.ph ], [ %754, %vector.body ]
  %vec.phi392 = phi <8 x i64> [ zeroinitializer, %vector.ph ], [ %755, %vector.body ]
  %vec.phi393 = phi <8 x i64> [ zeroinitializer, %vector.ph ], [ %756, %vector.body ]
  %step.add = add nuw <8 x i64> %vec.ind, splat (i64 8)
  %step.add.2 = add nuw <8 x i64> %vec.ind, splat (i64 16)
  %step.add.3 = add nuw <8 x i64> %vec.ind, splat (i64 24)
  %wide.gep = getelementptr inbounds nuw [160 x i8], ptr %749, <8 x i64> %vec.ind
  %wide.gep394 = getelementptr inbounds nuw [160 x i8], ptr %749, <8 x i64> %step.add
  %wide.gep395 = getelementptr inbounds nuw [160 x i8], ptr %749, <8 x i64> %step.add.2
  %wide.gep396 = getelementptr inbounds nuw [160 x i8], ptr %749, <8 x i64> %step.add.3
  %wide.gep397 = getelementptr i8, <8 x ptr> %wide.gep, i64 64
  %wide.gep398 = getelementptr i8, <8 x ptr> %wide.gep394, i64 64
  %wide.gep399 = getelementptr i8, <8 x ptr> %wide.gep395, i64 64
  %wide.gep400 = getelementptr i8, <8 x ptr> %wide.gep396, i64 64
  %wide.masked.gather = call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep397, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !23588
  %wide.masked.gather401 = call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep398, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !23588
  %wide.masked.gather402 = call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep399, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !23588
  %wide.masked.gather403 = call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep400, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !23588
  %753 = add <8 x i64> %wide.masked.gather, %vec.phi
  %754 = add <8 x i64> %wide.masked.gather401, %vec.phi391
  %755 = add <8 x i64> %wide.masked.gather402, %vec.phi392
  %756 = add <8 x i64> %wide.masked.gather403, %vec.phi393
  %index.next = add nuw i64 %index, 32
  %vec.ind.next = add nuw <8 x i64> %vec.ind, splat (i64 32)
  %757 = icmp eq i64 %index.next, %n.vec
  br i1 %757, label %middle.block, label %vector.body, !llvm.loop !23591

middle.block:                                     ; preds = %vector.body
  %bin.rdx = add <8 x i64> %754, %753
  %bin.rdx404 = add <8 x i64> %755, %bin.rdx
  %bin.rdx405 = add <8 x i64> %756, %bin.rdx404
  %758 = call i64 @llvm.vector.reduce.add.v8i64(<8 x i64> %bin.rdx405)
  %cmp.n = icmp eq i64 %751, %n.vec
  br i1 %cmp.n, label %.loopexit80, label %vec.epilog.iter.check

vec.epilog.iter.check:                            ; preds = %middle.block
  %min.epilog.iters.check = icmp eq i64 %n.mod.vf, 0
  br i1 %min.epilog.iters.check, label %.preheader79.preheader, label %vec.epilog.ph, !prof !11074

vec.epilog.ph:                                    ; preds = %vector.main.loop.iter.check, %vec.epilog.iter.check
  %vec.epilog.resume.val = phi i64 [ %n.vec, %vec.epilog.iter.check ], [ 0, %vector.main.loop.iter.check ]
  %bc.merge.rdx = phi i64 [ %758, %vec.epilog.iter.check ], [ 0, %vector.main.loop.iter.check ]
  %n.vec407 = and i64 %751, -8
  %759 = insertelement <8 x i64> <i64 poison, i64 0, i64 0, i64 0, i64 0, i64 0, i64 0, i64 0>, i64 %bc.merge.rdx, i64 0
  %broadcast.splatinsert = insertelement <8 x i64> poison, i64 %vec.epilog.resume.val, i64 0
  %broadcast.splat = shufflevector <8 x i64> %broadcast.splatinsert, <8 x i64> poison, <8 x i32> zeroinitializer
  %induction = or disjoint <8 x i64> %broadcast.splat, <i64 0, i64 1, i64 2, i64 3, i64 4, i64 5, i64 6, i64 7>
  br label %vec.epilog.vector.body

vec.epilog.vector.body:                           ; preds = %vec.epilog.vector.body, %vec.epilog.ph
  %index408 = phi i64 [ %vec.epilog.resume.val, %vec.epilog.ph ], [ %index.next414, %vec.epilog.vector.body ]
  %vec.ind409 = phi <8 x i64> [ %induction, %vec.epilog.ph ], [ %vec.ind.next415, %vec.epilog.vector.body ]
  %vec.phi410 = phi <8 x i64> [ %759, %vec.epilog.ph ], [ %760, %vec.epilog.vector.body ]
  %wide.gep411 = getelementptr inbounds nuw [160 x i8], ptr %749, <8 x i64> %vec.ind409
  %wide.gep412 = getelementptr i8, <8 x ptr> %wide.gep411, i64 64
  %wide.masked.gather413 = call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep412, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !23588
  %760 = add <8 x i64> %wide.masked.gather413, %vec.phi410
  %index.next414 = add nuw i64 %index408, 8
  %vec.ind.next415 = add nuw <8 x i64> %vec.ind409, splat (i64 8)
  %761 = icmp eq i64 %index.next414, %n.vec407
  br i1 %761, label %vec.epilog.middle.block, label %vec.epilog.vector.body, !llvm.loop !23592

vec.epilog.middle.block:                          ; preds = %vec.epilog.vector.body
  %762 = call i64 @llvm.vector.reduce.add.v8i64(<8 x i64> %760)
  %cmp.n416 = icmp eq i64 %751, %n.vec407
  br i1 %cmp.n416, label %.loopexit80, label %.preheader79.preheader

.preheader79.preheader:                           ; preds = %iter.check, %vec.epilog.iter.check, %vec.epilog.middle.block
  %.ph504 = phi i64 [ 0, %iter.check ], [ %n.vec, %vec.epilog.iter.check ], [ %n.vec407, %vec.epilog.middle.block ]
  %.ph505 = phi i64 [ 0, %iter.check ], [ %758, %vec.epilog.iter.check ], [ %762, %vec.epilog.middle.block ]
  br label %.preheader79

.preheader79:                                     ; preds = %.preheader79.preheader, %.preheader79
  %763 = phi i64 [ %770, %.preheader79 ], [ %.ph504, %.preheader79.preheader ]
  %764 = phi i64 [ %769, %.preheader79 ], [ %.ph505, %.preheader79.preheader ]
  %765 = getelementptr inbounds nuw [160 x i8], ptr %749, i64 %763
  %766 = getelementptr i8, ptr %765, i64 64
  %767 = load i64, ptr %766, align 8, !noalias !23588, !noundef !1740
  %768 = icmp ult i64 %767, 288230376151711744
  call void @llvm.assume(i1 %768), !noalias !23583
  %769 = add i64 %767, %764
  %770 = add nuw i64 %763, 1
  %771 = icmp eq i64 %770, %751
  br i1 %771, label %.loopexit80, label %.preheader79, !llvm.loop !23593

772:                                              ; preds = %.loopexit80
  %773 = landingpad { ptr, i32 }
          cleanup
  br label %1362

.loopexit80:                                      ; preds = %.preheader79, %middle.block, %vec.epilog.middle.block, %744
  %774 = phi i64 [ 0, %744 ], [ %762, %vec.epilog.middle.block ], [ %758, %middle.block ], [ %769, %.preheader79 ]
  %775 = getelementptr inbounds nuw i8, ptr %15, i64 24
  %776 = load i8, ptr %775, align 8, !range !1747, !noundef !1740
  %777 = trunc nuw i8 %776 to i1
  call void @llvm.lifetime.start.p0(ptr nonnull %17)
  call void @llvm.lifetime.start.p0(ptr nonnull %16), !noalias !23570
  %778 = getelementptr inbounds nuw i8, ptr %51, i64 195
  %779 = load i8, ptr %778, align 1, !range !6143, !alias.scope !23577, !noalias !23587, !noundef !1740
; invoke purrdf_sparql_eval::row_checkpoint::commit_items::<purrdf_core::ir::dataset::RdfDataset, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, <purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::commit_rows<purrdf_core::ir::dataset::RdfDataset, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>::{closure#0}>
  invoke fastcc void @purrdf_sparql_eval::row_checkpoint::commit_items::<purrdf_core::ir::dataset::RdfDataset, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, <purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::commit_rows<purrdf_core::ir::dataset::RdfDataset, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>::{closure#0}>(ptr noalias nofree noundef align 16 captures(none) dereferenceable(96) %16, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %4, i8 noundef %779, ptr noalias nofree noundef align 8 captures(address) dereferenceable(32) %15, ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %14, i64 noundef %747)
          to label %780 unwind label %772

780:                                              ; preds = %.loopexit80
  %781 = load i64, ptr %16, align 16, !range !2527, !noalias !23570, !noundef !1740
  %782 = icmp eq i64 %781, -1
  %783 = getelementptr inbounds nuw i8, ptr %16, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %17, ptr noundef nonnull align 8 dereferenceable(24) %783, i64 24, i1 false), !noalias !23570
  %784 = getelementptr inbounds nuw i8, ptr %16, i64 32
  br i1 %782, label %785, label %796

785:                                              ; preds = %780
  %786 = load i8, ptr %784, align 16, !noalias !23570
  call void @llvm.lifetime.end.p0(ptr nonnull %16), !noalias !23570
  call void @llvm.lifetime.start.p0(ptr nonnull %13), !noalias !23570
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %13, ptr noundef nonnull align 8 dereferenceable(24) %17, i64 24, i1 false), !noalias !23570
  call void @llvm.lifetime.end.p0(ptr nonnull %17)
; invoke purrdf_sparql_eval::row_checkpoint::settle_commit::<purrdf_core::ir::dataset::RdfDataset>
  %787 = invoke fastcc { i64, i64 } @purrdf_sparql_eval::row_checkpoint::settle_commit::<purrdf_core::ir::dataset::RdfDataset>(ptr noundef nonnull align 16 dereferenceable(1248) %4, i8 noundef %786, i1 noundef zeroext %777, i64 noundef %774)
          to label %790 unwind label %788, !noalias !23594

788:                                              ; preds = %785
  %789 = landingpad { ptr, i32 }
          cleanup
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %13) #89, !noalias !23594
  br label %1362

790:                                              ; preds = %785
  %791 = extractvalue { i64, i64 } %787, 0
  %792 = extractvalue { i64, i64 } %787, 1
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %37, ptr noundef nonnull align 8 dereferenceable(24) %13, i64 24, i1 false), !noalias !23595
  call void @llvm.lifetime.end.p0(ptr nonnull %13), !noalias !23570
  call void @llvm.lifetime.end.p0(ptr nonnull %20), !noalias !23570
  %793 = trunc nuw i64 %791 to i1
  br label %803

794:                                              ; preds = %713, %654
  %795 = phi { ptr, i32 } [ %714, %713 ], [ %655, %654 ]
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %14) #89
  br label %1362

796:                                              ; preds = %780
  %797 = getelementptr inbounds nuw i8, ptr %16, i64 48
  %798 = getelementptr inbounds nuw i8, ptr %0, i64 64
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(48) %798, ptr noundef nonnull align 16 dereferenceable(48) %797, i64 48, i1 false), !noalias !23505
  %799 = getelementptr inbounds nuw i8, ptr %0, i64 24
  %800 = getelementptr inbounds nuw i8, ptr %0, i64 16
  %801 = getelementptr inbounds nuw i8, ptr %0, i64 48
  %802 = load <2 x i64>, ptr %784, align 16, !noalias !23570
  call void @llvm.lifetime.end.p0(ptr nonnull %16), !noalias !23570
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %37, ptr noundef nonnull align 8 dereferenceable(24) %17, i64 24, i1 false), !noalias !23595
  call void @llvm.lifetime.end.p0(ptr nonnull %17)
  call void @llvm.lifetime.end.p0(ptr nonnull %20), !noalias !23570
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %38, ptr noundef nonnull align 8 dereferenceable(24) %37, i64 24, i1 false), !noalias !23464
  call void @llvm.lifetime.end.p0(ptr nonnull %37)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %799, ptr noundef nonnull align 8 dereferenceable(24) %38, i64 24, i1 false), !noalias !23505
  store i64 %781, ptr %800, align 16, !alias.scope !23457, !noalias !23505
  store <2 x i64> %802, ptr %801, align 16, !alias.scope !23457, !noalias !23505
  store i64 1, ptr %0, align 16, !alias.scope !23457, !noalias !23505
  call void @llvm.lifetime.end.p0(ptr nonnull %38)
  call void @llvm.lifetime.end.p0(ptr nonnull %42), !noalias !23464
; invoke core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>>(ptr noalias nofree noundef align 8 dereferenceable(32) %43)
          to label %1135 unwind label %624, !noalias !23469

803:                                              ; preds = %790, %634
  %804 = phi i64 [ %792, %790 ], [ undef, %634 ]
  %805 = phi i1 [ %793, %790 ], [ false, %634 ]
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %38, ptr noundef nonnull align 8 dereferenceable(24) %37, i64 24, i1 false), !noalias !23464
  call void @llvm.lifetime.end.p0(ptr nonnull %37)
  call void @llvm.lifetime.start.p0(ptr nonnull %39), !noalias !23464
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %39, ptr noundef nonnull align 8 dereferenceable(24) %38, i64 24, i1 false), !noalias !23464
  call void @llvm.lifetime.end.p0(ptr nonnull %38)
  %806 = load i64, ptr %43, align 8, !noalias !23464
  %807 = getelementptr inbounds nuw i8, ptr %43, i64 8
  %808 = load i64, ptr %807, align 8, !noalias !23464
  %809 = getelementptr inbounds nuw i8, ptr %43, i64 16
  %810 = load i64, ptr %809, align 8, !noalias !23464
  %811 = getelementptr inbounds nuw i8, ptr %43, i64 24
  %812 = load i64, ptr %811, align 8, !noalias !23464
  %813 = icmp ugt i64 %806, 2
  %814 = select i1 %813, i64 %810, i64 %806
  %815 = add i64 %814, -1
  %816 = select i1 %813, i64 %806, i64 1
  %817 = select i1 %813, i64 1, i64 %810
  call void @llvm.lifetime.start.p0(ptr nonnull %12), !noalias !23596
  store i64 %816, ptr %12, align 8, !noalias !23600
  %818 = getelementptr inbounds nuw i8, ptr %12, i64 8
  store i64 %808, ptr %818, align 8, !noalias !23600
  %819 = getelementptr inbounds nuw i8, ptr %12, i64 16
  store i64 %817, ptr %819, align 8, !noalias !23600
  %820 = getelementptr inbounds nuw i8, ptr %12, i64 24
  store i64 %812, ptr %820, align 8, !noalias !23600
  %821 = getelementptr inbounds nuw i8, ptr %12, i64 32
  store i64 0, ptr %821, align 8, !noalias !23596
  %822 = getelementptr inbounds nuw i8, ptr %12, i64 40
  store i64 %815, ptr %822, align 8, !noalias !23596
  %823 = icmp eq i64 %815, 0
  br i1 %823, label %.loopexit78, label %824

824:                                              ; preds = %803
  %825 = inttoptr i64 %808 to ptr
  %826 = select i1 %813, ptr %825, ptr %818
  %827 = getelementptr inbounds nuw i8, ptr %4, i64 640
  br label %830

828:                                              ; preds = %830
  %829 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<purrdf_core::small::IntoIter<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_core::small::IntoIter<[purrdf_sparql_eval::witness::RelationWitness; 1]>>(ptr noalias nofree noundef align 8 dereferenceable(48) %12) #89
          to label %838 unwind label %836, !noalias !23601

830:                                              ; preds = %834, %824
  %831 = phi i64 [ 0, %824 ], [ %832, %834 ]
  %832 = add nuw i64 %831, 1
  store i64 %832, ptr %821, align 8, !alias.scope !23602, !noalias !23605
  %833 = getelementptr inbounds nuw [24 x i8], ptr %826, i64 %831
  call void @llvm.lifetime.start.p0(ptr nonnull %11), !noalias !23596
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %11, ptr noundef nonnull align 8 dereferenceable(24) %833, i64 24, i1 false), !noalias !23601
; invoke <purrdf_sparql_eval::witness::RelationWitness>::merge
  invoke void @<purrdf_sparql_eval::witness::RelationWitness>::merge(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %827, ptr noalias nofree noundef nonnull readonly align 8 captures(none) dereferenceable(24) %11)
          to label %834 unwind label %828, !noalias !23601

.loopexit78:                                      ; preds = %834, %803
; invoke core::ptr::drop_glue::<purrdf_core::small::IntoIter<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_core::small::IntoIter<[purrdf_sparql_eval::witness::RelationWitness; 1]>>(ptr noalias nofree noundef align 8 dereferenceable(48) %12)
          to label %842 unwind label %840

834:                                              ; preds = %830
  call void @llvm.lifetime.end.p0(ptr nonnull %11), !noalias !23596
  %835 = icmp eq i64 %832, %815
  br i1 %835, label %.loopexit78, label %830

836:                                              ; preds = %828
  %837 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #90, !noalias !23601
  unreachable

838:                                              ; preds = %915, %840, %828
  %839 = phi { ptr, i32 } [ %829, %828 ], [ %841, %840 ], [ %916, %915 ]
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %39) #89, !noalias !23469
  br label %571

840:                                              ; preds = %.loopexit77, %959, %892, %.loopexit78
  %841 = landingpad { ptr, i32 }
          cleanup
  br label %838

842:                                              ; preds = %.loopexit78
  call void @llvm.lifetime.end.p0(ptr nonnull %12), !noalias !23596
  br i1 %805, label %843, label %845

843:                                              ; preds = %842
  %844 = icmp ugt i64 %804, %188
  br i1 %844, label %892, label %852, !prof !1742

845:                                              ; preds = %1136, %842
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %49, ptr noundef nonnull align 8 dereferenceable(24) %39, i64 24, i1 false), !noalias !23464
  call void @llvm.lifetime.end.p0(ptr nonnull %39), !noalias !23464
  call void @llvm.lifetime.end.p0(ptr nonnull %42), !noalias !23464
  call void @llvm.lifetime.end.p0(ptr nonnull %43), !noalias !23464
  call void @llvm.experimental.noalias.scope.decl(metadata !23607)
  %846 = load ptr, ptr %48, align 8, !alias.scope !23607, !noalias !23469, !noundef !1740
  %847 = icmp eq ptr %846, null
  br i1 %847, label %1138, label %848

848:                                              ; preds = %845
  %849 = atomicrmw sub ptr %846, i64 1 release, align 8, !noalias !23610
  %850 = icmp eq i64 %849, 1
  br i1 %850, label %851, label %1138

851:                                              ; preds = %848
  fence acquire, !noalias !23469
; invoke <alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %48) #91
          to label %1138 unwind label %533, !inline_history !2025

852:                                              ; preds = %843
  call void @llvm.lifetime.start.p0(ptr nonnull %36), !noalias !23464
  call void @llvm.experimental.noalias.scope.decl(metadata !23613)
  %853 = load ptr, ptr %560, align 8, !noalias !23616, !noundef !1740
  %854 = icmp eq ptr %853, null
  br i1 %854, label %878, label %855

855:                                              ; preds = %852
  %856 = getelementptr inbounds nuw i8, ptr %853, i64 16
  %857 = load i64, ptr %856, align 8, !noalias !23469
  %858 = icmp ne i64 %857, -1
  %859 = getelementptr inbounds nuw i8, ptr %853, i64 40
  %860 = load i64, ptr %859, align 8, !noalias !23469
  %861 = icmp ne i64 %860, -1
  %862 = getelementptr inbounds nuw i8, ptr %853, i64 336
  %863 = load ptr, ptr %862, align 8, !noalias !23469, !noundef !1740
  %864 = icmp ne ptr %863, null
  %865 = select i1 %864, i1 true, i1 %858
  %866 = select i1 %865, i1 true, i1 %861
  %867 = getelementptr inbounds nuw i8, ptr %36, i64 192
  %868 = getelementptr inbounds nuw i8, ptr %36, i64 160
  %869 = getelementptr inbounds nuw i8, ptr %36, i64 184
  %870 = getelementptr inbounds nuw i8, ptr %36, i64 8
  %871 = getelementptr inbounds nuw i8, ptr %36, i64 16
  %872 = getelementptr inbounds nuw i8, ptr %36, i64 32
  %873 = getelementptr inbounds nuw i8, ptr %36, i64 40
  %874 = getelementptr inbounds nuw i8, ptr %36, i64 56
  %875 = getelementptr inbounds nuw i8, ptr %36, i64 64
  %876 = getelementptr inbounds nuw i8, ptr %36, i64 72
  %877 = getelementptr inbounds nuw i8, ptr %36, i64 88
  br i1 %866, label %891, label %890

878:                                              ; preds = %852
  %879 = getelementptr inbounds nuw i8, ptr %36, i64 192
  %880 = getelementptr inbounds nuw i8, ptr %36, i64 160
  %881 = getelementptr inbounds nuw i8, ptr %36, i64 184
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %880, i8 0, i64 24, i1 false), !alias.scope !23613, !noalias !23469
  store i64 -1, ptr %881, align 8, !alias.scope !23613, !noalias !23469
  store <4 x i8> <i8 0, i8 0, i8 0, i8 4>, ptr %879, align 8, !alias.scope !23613, !noalias !23469
  store i64 0, ptr %36, align 8, !alias.scope !23613, !noalias !23469
  %882 = getelementptr inbounds nuw i8, ptr %36, i64 8
  store ptr inttoptr (i64 8 to ptr), ptr %882, align 8, !alias.scope !23613, !noalias !23469
  %883 = getelementptr inbounds nuw i8, ptr %36, i64 16
  %884 = getelementptr inbounds nuw i8, ptr %36, i64 32
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %883, i8 0, i64 16, i1 false), !alias.scope !23613, !noalias !23469
  store ptr inttoptr (i64 8 to ptr), ptr %884, align 8, !alias.scope !23613, !noalias !23469
  %885 = getelementptr inbounds nuw i8, ptr %36, i64 40
  %886 = getelementptr inbounds nuw i8, ptr %36, i64 56
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %885, i8 0, i64 16, i1 false), !alias.scope !23613, !noalias !23469
  store ptr inttoptr (i64 8 to ptr), ptr %886, align 8, !alias.scope !23613, !noalias !23469
  %887 = getelementptr inbounds nuw i8, ptr %36, i64 64
  store i64 0, ptr %887, align 8, !alias.scope !23613, !noalias !23469
  %888 = getelementptr inbounds nuw i8, ptr %36, i64 72
  %889 = getelementptr inbounds nuw i8, ptr %36, i64 88
  br label %893

890:                                              ; preds = %855
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %868, i8 0, i64 24, i1 false), !alias.scope !23613, !noalias !23469
  store i64 -1, ptr %869, align 8, !alias.scope !23613, !noalias !23469
  store <4 x i8> <i8 0, i8 0, i8 0, i8 4>, ptr %867, align 8, !alias.scope !23613, !noalias !23469
  store i64 0, ptr %36, align 8, !alias.scope !23613, !noalias !23469
  store ptr inttoptr (i64 8 to ptr), ptr %870, align 8, !alias.scope !23613, !noalias !23469
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %871, i8 0, i64 16, i1 false), !alias.scope !23613, !noalias !23469
  store ptr inttoptr (i64 8 to ptr), ptr %872, align 8, !alias.scope !23613, !noalias !23469
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %873, i8 0, i64 16, i1 false), !alias.scope !23613, !noalias !23469
  store ptr inttoptr (i64 8 to ptr), ptr %874, align 8, !alias.scope !23613, !noalias !23469
  store i64 0, ptr %875, align 8, !alias.scope !23613, !noalias !23469
  br label %893

891:                                              ; preds = %855
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %868, i8 0, i64 24, i1 false), !alias.scope !23613, !noalias !23469
  store i64 -1, ptr %869, align 8, !alias.scope !23613, !noalias !23469
  store <4 x i8> <i8 0, i8 0, i8 1, i8 4>, ptr %867, align 8, !alias.scope !23613, !noalias !23469
  store i64 0, ptr %36, align 8, !alias.scope !23613, !noalias !23469
  store ptr inttoptr (i64 8 to ptr), ptr %870, align 8, !alias.scope !23613, !noalias !23469
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %871, i8 0, i64 16, i1 false), !alias.scope !23613, !noalias !23469
  store ptr inttoptr (i64 8 to ptr), ptr %872, align 8, !alias.scope !23613, !noalias !23469
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %873, i8 0, i64 16, i1 false), !alias.scope !23613, !noalias !23469
  store ptr inttoptr (i64 8 to ptr), ptr %874, align 8, !alias.scope !23613, !noalias !23469
  store i64 0, ptr %875, align 8, !alias.scope !23613, !noalias !23469
  br label %893

892:                                              ; preds = %843
; invoke core::slice::index::slice_index_fail
  invoke void @core::slice::index::slice_index_fail(i64 noundef %804, i64 noundef %188, i64 noundef %188, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.e5162873a9a3251d11c4df37a70e4654.405) #93
          to label %1137 unwind label %840, !noalias !23469

893:                                              ; preds = %891, %890, %878
  %894 = phi ptr [ %876, %891 ], [ %876, %890 ], [ %888, %878 ]
  %895 = phi ptr [ %877, %891 ], [ %877, %890 ], [ %889, %878 ]
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %894, i8 -1, i64 16, i1 false), !alias.scope !23613, !noalias !23469
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(65) %895, i8 0, i64 65, i1 false), !alias.scope !23613, !noalias !23469
  %896 = getelementptr inbounds nuw [40 x i8], ptr %557, i64 %188
  %897 = icmp samesign eq i64 %804, %188
  br i1 %897, label %.loopexit77, label %898

898:                                              ; preds = %893
  %899 = getelementptr inbounds nuw [40 x i8], ptr %557, i64 %804
  %900 = getelementptr inbounds nuw i8, ptr %10, i64 8
  %901 = getelementptr inbounds nuw i8, ptr %10, i64 16
  %902 = getelementptr inbounds nuw i8, ptr %10, i64 24
  %903 = getelementptr inbounds nuw i8, ptr %39, i64 16
  %904 = getelementptr inbounds nuw i8, ptr %39, i64 8
  %905 = getelementptr inbounds nuw i8, ptr %7, i64 8
  %906 = getelementptr inbounds nuw i8, ptr %7, i64 12
  %907 = getelementptr inbounds nuw i8, ptr %6, i64 8
  %wide.gep466 = getelementptr inbounds nuw [8 x i8], ptr %900, <4 x i64> <i64 0, i64 1, i64 2, i64 3>
  %wide.gep467 = getelementptr inbounds nuw i8, <4 x ptr> %wide.gep466, i64 4
  br label %908

908:                                              ; preds = %1111, %898
  %909 = phi ptr [ %899, %898 ], [ %910, %1111 ]
  %910 = getelementptr inbounds nuw i8, ptr %909, i64 40
  call void @llvm.lifetime.start.p0(ptr nonnull %35), !noalias !23464
; invoke <purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::pass::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::pass::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %35, ptr noalias nofree noundef align 8 dereferenceable(200) %36, ptr noundef nonnull align 16 dereferenceable(1248) %4)
          to label %917 unwind label %911, !noalias !23469

911:                                              ; preds = %948, %930, %908
  %912 = landingpad { ptr, i32 }
          cleanup
  br label %915

913:                                              ; preds = %1002
  %914 = landingpad { ptr, i32 }
          cleanup
  br label %915

915:                                              ; preds = %1124, %1121, %913, %911
  %916 = phi { ptr, i32 } [ %1122, %1121 ], [ %1122, %1124 ], [ %912, %911 ], [ %914, %913 ]
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(200) %36)
          to label %838 unwind label %550

917:                                              ; preds = %908
  %918 = load i8, ptr %35, align 8, !range !1743, !noalias !23464, !noundef !1740
  %919 = icmp eq i8 %918, -1
  call void @llvm.lifetime.end.p0(ptr nonnull %35), !noalias !23464
  br i1 %919, label %920, label %.loopexit77

920:                                              ; preds = %917
  call void @llvm.lifetime.start.p0(ptr nonnull %34)
  %921 = load i64, ptr %909, align 8, !range !1778, !noalias !23469, !noundef !1740
  %922 = add i64 %921, -1
  %923 = icmp ugt i64 %922, 4
  %924 = getelementptr inbounds nuw i8, ptr %909, i64 8
  br i1 %923, label %925, label %930

925:                                              ; preds = %920
  %926 = load ptr, ptr %924, align 8, !noalias !23469, !nonnull !1740, !noundef !1740
  %927 = getelementptr inbounds nuw i8, ptr %909, i64 16
  %928 = load i64, ptr %927, align 8, !noalias !23469, !noundef !1740
  %929 = add i64 %928, -1
  br label %930

930:                                              ; preds = %925, %920
  %931 = phi i64 [ %929, %925 ], [ %922, %920 ]
  %932 = phi ptr [ %926, %925 ], [ %924, %920 ]
  %933 = load ptr, ptr %52, align 8, !noalias !23464, !nonnull !1740, !noundef !1740
  %934 = getelementptr inbounds nuw i8, ptr %933, i64 16
  call void @llvm.lifetime.start.p0(ptr nonnull %7), !noalias !23617
; invoke <purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::term::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::term::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef align 16 captures(none) dereferenceable(96) %7, ptr noalias nofree noundef nonnull align 8 dereferenceable(216) %50, ptr noalias nofree noundef nonnull readonly align 4 captures(address, read_provenance) %932, i64 noundef range(i64 0, 1152921504606846976) %931, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(56) %934, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %4)
          to label %935 unwind label %911, !inline_history !23500

935:                                              ; preds = %930
  %936 = load i64, ptr %7, align 16, !range !2527, !noalias !23617, !noundef !1740
  %937 = icmp eq i64 %936, -1
  %938 = load i32, ptr %905, align 8, !noalias !23617
  %939 = load i32, ptr %906, align 4, !noalias !23617
  br i1 %937, label %945, label %940

940:                                              ; preds = %935
  %941 = getelementptr inbounds nuw i8, ptr %7, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(80) %34, ptr noundef nonnull align 16 dereferenceable(80) %941, i64 80, i1 false), !noalias !23624
  call void @llvm.lifetime.end.p0(ptr nonnull %7), !noalias !23617
  %942 = trunc i32 %938 to i8
  %943 = lshr i32 %938, 8
  %944 = trunc nuw i32 %943 to i24
  br label %959

945:                                              ; preds = %935
  call void @llvm.lifetime.end.p0(ptr nonnull %7), !noalias !23617
  %946 = icmp eq i32 %938, 2
  br i1 %946, label %947, label %948

947:                                              ; preds = %945
  call void @llvm.lifetime.end.p0(ptr nonnull %34)
  br label %1111

948:                                              ; preds = %945
  call void @llvm.lifetime.start.p0(ptr nonnull %6), !noalias !23617
; invoke purrdf_sparql_eval::expr::ebv_term::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @purrdf_sparql_eval::expr::ebv_term::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef align 16 captures(none) dereferenceable(96) %6, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %4, i32 noundef %938, i32 noundef %939)
          to label %949 unwind label %911, !inline_history !23500

949:                                              ; preds = %948
  %950 = load i64, ptr %6, align 16, !range !2527, !noalias !23617, !noundef !1740
  %951 = icmp eq i64 %950, -1
  %952 = load i8, ptr %907, align 8, !noalias !23617
  br i1 %951, label %969, label %953

953:                                              ; preds = %949
  %954 = getelementptr inbounds nuw i8, ptr %6, i64 9
  %955 = load i24, ptr %954, align 1, !noalias !23624
  %956 = getelementptr inbounds nuw i8, ptr %6, i64 12
  %957 = load i32, ptr %956, align 4, !noalias !23624
  %958 = getelementptr inbounds nuw i8, ptr %6, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(80) %34, ptr noundef nonnull align 16 dereferenceable(80) %958, i64 80, i1 false), !noalias !23624
  call void @llvm.lifetime.end.p0(ptr nonnull %6), !noalias !23617
  br label %959

959:                                              ; preds = %953, %940
  %960 = phi i24 [ %944, %940 ], [ %955, %953 ]
  %961 = phi i8 [ %942, %940 ], [ %952, %953 ]
  %962 = phi i32 [ %939, %940 ], [ %957, %953 ]
  %963 = phi i64 [ %936, %940 ], [ %950, %953 ]
  %964 = getelementptr inbounds nuw i8, ptr %0, i64 25
  store i24 %960, ptr %964, align 1, !noalias !23505
  %965 = getelementptr inbounds nuw i8, ptr %0, i64 28
  store i32 %962, ptr %965, align 4, !noalias !23505
  %966 = getelementptr inbounds nuw i8, ptr %0, i64 32
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(80) %966, ptr noundef nonnull align 16 dereferenceable(80) %34, i64 80, i1 false), !noalias !23505
  call void @llvm.lifetime.end.p0(ptr nonnull %34)
  %967 = getelementptr inbounds nuw i8, ptr %0, i64 16
  store i64 %963, ptr %967, align 16, !alias.scope !23457, !noalias !23505
  %968 = getelementptr inbounds nuw i8, ptr %0, i64 24
  store i8 %961, ptr %968, align 8, !alias.scope !23457, !noalias !23505
  store i64 1, ptr %0, align 16, !alias.scope !23457, !noalias !23505
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(200) %36)
          to label %1134 unwind label %840

969:                                              ; preds = %949
  call void @llvm.lifetime.end.p0(ptr nonnull %6), !noalias !23617
  call void @llvm.lifetime.end.p0(ptr nonnull %34)
  %970 = and i8 %952, 1
  %971 = icmp eq i8 %970, 0
  br i1 %971, label %1111, label %972

972:                                              ; preds = %969
  call void @llvm.lifetime.start.p0(ptr nonnull %33)
  call void @llvm.experimental.noalias.scope.decl(metadata !23625)
  %973 = load i64, ptr %909, align 8, !range !1778, !alias.scope !23625, !noalias !23628, !noundef !1740
  %974 = add i64 %973, -1
  %975 = icmp ugt i64 %974, 4
  %976 = getelementptr inbounds nuw i8, ptr %909, i64 16
  %977 = load i64, ptr %976, align 8, !alias.scope !23625, !noalias !23628
  %978 = add i64 %977, -1
  %979 = select i1 %975, i64 %978, i64 %974
  %980 = icmp ugt i64 %979, 4
  br i1 %980, label %990, label %981

981:                                              ; preds = %972
  call void @llvm.lifetime.start.p0(ptr nonnull %10), !noalias !23630
  %982 = icmp ugt i64 %973, 5
  %983 = load ptr, ptr %924, align 8, !alias.scope !23625, !noalias !23628, !nonnull !1740
  %984 = select i1 %982, ptr %983, ptr %924
  %985 = icmp eq i64 %979, 0
  br i1 %985, label %998, label %vector.body459

vector.body459:                                   ; preds = %981
  %trip.count.minus.1 = add nsw i64 %979, -1
  %broadcast.splatinsert457 = insertelement <4 x i64> poison, i64 %trip.count.minus.1, i64 0
  %broadcast.splat458 = shufflevector <4 x i64> %broadcast.splatinsert457, <4 x i64> poison, <4 x i32> zeroinitializer
  %986 = icmp uge <4 x i64> %broadcast.splat458, <i64 0, i64 1, i64 2, i64 3>
  %wide.gep462 = getelementptr inbounds nuw [8 x i8], ptr %984, <4 x i64> <i64 0, i64 1, i64 2, i64 3>
  %wide.masked.gather463 = call <4 x i32> @llvm.masked.gather.v4i32.v4p0(<4 x ptr> align 4 %wide.gep462, <4 x i1> %986, <4 x i32> poison), !noalias !23628
  %wide.gep464 = getelementptr i8, <4 x ptr> %wide.gep462, i64 4
  %wide.masked.gather465 = call <4 x i32> @llvm.masked.gather.v4i32.v4p0(<4 x ptr> align 4 %wide.gep464, <4 x i1> %986, <4 x i32> poison), !noalias !23628
  %987 = icmp eq <4 x i32> %wide.masked.gather463, splat (i32 2)
  %988 = select <4 x i1> %987, <4 x i32> undef, <4 x i32> %wide.masked.gather465
  call void @llvm.masked.scatter.v4i32.v4p0(<4 x i32> %wide.masked.gather463, <4 x ptr> align 4 %wide.gep466, <4 x i1> %986), !noalias !23630
  call void @llvm.masked.scatter.v4i32.v4p0(<4 x i32> %988, <4 x ptr> align 4 %wide.gep467, <4 x i1> %986), !noalias !23630
  %989 = add nuw nsw i64 %979, 1
  br label %998

990:                                              ; preds = %972
  %991 = shl i64 %979, 3
  %992 = icmp ugt i64 %979, 2305843009213693951
  %993 = icmp ugt i64 %991, 9223372036854775804
  %994 = or i1 %992, %993
  br i1 %994, label %1002, label %995, !prof !6193

995:                                              ; preds = %990
; call __rustc::__rust_alloc
  %996 = call noundef align 4 ptr @__rustc::__rust_alloc(i64 noundef %991, i64 noundef range(i64 1, -9223372036854775807) 4) #92, !noalias !23631
  %997 = icmp eq ptr %996, null
  br i1 %997, label %1002, label %iter.check436

998:                                              ; preds = %vector.body459, %981
  %999 = phi i64 [ 1, %981 ], [ %989, %vector.body459 ]
  store i64 %999, ptr %10, align 8, !noalias !23630
  %1000 = load ptr, ptr %900, align 8, !noalias !23634
  %1001 = load i64, ptr %901, align 8, !noalias !23634
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %33, ptr noundef nonnull align 8 dereferenceable(16) %902, i64 16, i1 false), !noalias !23634
  call void @llvm.lifetime.end.p0(ptr nonnull %10), !noalias !23630
  br label %1113

1002:                                             ; preds = %995, %990
  %1003 = phi i64 [ 4, %995 ], [ 0, %990 ]
; invoke alloc::raw_vec::handle_error
  invoke void @alloc::raw_vec::handle_error(i64 noundef %1003, i64 %991) #93
          to label %1004 unwind label %913

1004:                                             ; preds = %1002
  unreachable

iter.check436:                                    ; preds = %995
  %1005 = load ptr, ptr %924, align 8, !alias.scope !23625, !noalias !23628, !nonnull !1740
  %1006 = select i1 %975, ptr %1005, ptr %924
  %min.iters.check419 = icmp ult i64 %979, 8
  br i1 %min.iters.check419, label %vec.epilog.scalar.ph437.preheader, label %vector.memcheck

vector.memcheck:                                  ; preds = %iter.check436
  %scevgep = getelementptr i8, ptr %996, i64 %991
  %scevgep418 = getelementptr i8, ptr %1006, i64 %991
  %bound0 = icmp ult ptr %996, %scevgep418
  %bound1 = icmp ult ptr %1006, %scevgep
  %found.conflict = and i1 %bound0, %bound1
  br i1 %found.conflict, label %vec.epilog.scalar.ph437.preheader, label %vector.main.loop.iter.check420

vector.main.loop.iter.check420:                   ; preds = %vector.memcheck
  %min.iters.check421 = icmp ult i64 %979, 32
  br i1 %min.iters.check421, label %vec.epilog.ph440, label %vector.ph422

vector.ph422:                                     ; preds = %vector.main.loop.iter.check420
  %n.mod.vf423 = and i64 %979, 24
  %n.vec424 = and i64 %979, 2305843009213693920
  br label %vector.body425

vector.body425:                                   ; preds = %vector.body425, %vector.ph422
  %index426 = phi i64 [ 0, %vector.ph422 ], [ %index.next432, %vector.body425 ]
  %1007 = or disjoint i64 %index426, 16
  %1008 = getelementptr inbounds nuw [8 x i8], ptr %1006, i64 %index426
  %1009 = getelementptr inbounds nuw [8 x i8], ptr %1006, i64 %1007
  %wide.vec = load <32 x i32>, ptr %1008, align 4, !alias.scope !23635, !noalias !23638
  %strided.vec = shufflevector <32 x i32> %wide.vec, <32 x i32> poison, <16 x i32> <i32 0, i32 2, i32 4, i32 6, i32 8, i32 10, i32 12, i32 14, i32 16, i32 18, i32 20, i32 22, i32 24, i32 26, i32 28, i32 30>
  %strided.vec427 = shufflevector <32 x i32> %wide.vec, <32 x i32> poison, <16 x i32> <i32 1, i32 3, i32 5, i32 7, i32 9, i32 11, i32 13, i32 15, i32 17, i32 19, i32 21, i32 23, i32 25, i32 27, i32 29, i32 31>
  %wide.vec428 = load <32 x i32>, ptr %1009, align 4, !alias.scope !23635, !noalias !23638
  %strided.vec429 = shufflevector <32 x i32> %wide.vec428, <32 x i32> poison, <16 x i32> <i32 0, i32 2, i32 4, i32 6, i32 8, i32 10, i32 12, i32 14, i32 16, i32 18, i32 20, i32 22, i32 24, i32 26, i32 28, i32 30>
  %strided.vec430 = shufflevector <32 x i32> %wide.vec428, <32 x i32> poison, <16 x i32> <i32 1, i32 3, i32 5, i32 7, i32 9, i32 11, i32 13, i32 15, i32 17, i32 19, i32 21, i32 23, i32 25, i32 27, i32 29, i32 31>
  %1010 = icmp eq <16 x i32> %strided.vec, splat (i32 2)
  %1011 = icmp eq <16 x i32> %strided.vec429, splat (i32 2)
  %1012 = select <16 x i1> %1010, <16 x i32> undef, <16 x i32> %strided.vec427
  %1013 = select <16 x i1> %1011, <16 x i32> undef, <16 x i32> %strided.vec430
  %1014 = getelementptr inbounds nuw [8 x i8], ptr %996, i64 %index426
  %1015 = getelementptr inbounds nuw [8 x i8], ptr %996, i64 %1007
  %interleaved.vec = shufflevector <16 x i32> %strided.vec, <16 x i32> %1012, <32 x i32> <i32 0, i32 16, i32 1, i32 17, i32 2, i32 18, i32 3, i32 19, i32 4, i32 20, i32 5, i32 21, i32 6, i32 22, i32 7, i32 23, i32 8, i32 24, i32 9, i32 25, i32 10, i32 26, i32 11, i32 27, i32 12, i32 28, i32 13, i32 29, i32 14, i32 30, i32 15, i32 31>
  store <32 x i32> %interleaved.vec, ptr %1014, align 4, !alias.scope !23651, !noalias !23653
  %interleaved.vec431 = shufflevector <16 x i32> %strided.vec429, <16 x i32> %1013, <32 x i32> <i32 0, i32 16, i32 1, i32 17, i32 2, i32 18, i32 3, i32 19, i32 4, i32 20, i32 5, i32 21, i32 6, i32 22, i32 7, i32 23, i32 8, i32 24, i32 9, i32 25, i32 10, i32 26, i32 11, i32 27, i32 12, i32 28, i32 13, i32 29, i32 14, i32 30, i32 15, i32 31>
  store <32 x i32> %interleaved.vec431, ptr %1015, align 4, !alias.scope !23651, !noalias !23653
  %index.next432 = add nuw i64 %index426, 32
  %1016 = icmp eq i64 %index.next432, %n.vec424
  br i1 %1016, label %middle.block433, label %vector.body425, !llvm.loop !23660

middle.block433:                                  ; preds = %vector.body425
  %ind.escape = add nsw i64 %n.vec424, -1
  %cmp.n434 = icmp eq i64 %979, %n.vec424
  br i1 %cmp.n434, label %.loopexit471, label %vec.epilog.iter.check438

vec.epilog.iter.check438:                         ; preds = %middle.block433
  %min.epilog.iters.check439 = icmp eq i64 %n.mod.vf423, 0
  br i1 %min.epilog.iters.check439, label %vec.epilog.scalar.ph437.preheader, label %vec.epilog.ph440, !prof !11074

vec.epilog.ph440:                                 ; preds = %vector.main.loop.iter.check420, %vec.epilog.iter.check438
  %vec.epilog.resume.val435 = phi i64 [ %n.vec424, %vec.epilog.iter.check438 ], [ 0, %vector.main.loop.iter.check420 ]
  %n.vec442 = and i64 %979, 2305843009213693944
  br label %vec.epilog.vector.body443

vec.epilog.vector.body443:                        ; preds = %vec.epilog.vector.body443, %vec.epilog.ph440
  %index444 = phi i64 [ %vec.epilog.resume.val435, %vec.epilog.ph440 ], [ %index.next449, %vec.epilog.vector.body443 ]
  %1017 = getelementptr inbounds nuw [8 x i8], ptr %1006, i64 %index444
  %wide.vec445 = load <16 x i32>, ptr %1017, align 4, !alias.scope !23635, !noalias !23638
  %strided.vec446 = shufflevector <16 x i32> %wide.vec445, <16 x i32> poison, <8 x i32> <i32 0, i32 2, i32 4, i32 6, i32 8, i32 10, i32 12, i32 14>
  %strided.vec447 = shufflevector <16 x i32> %wide.vec445, <16 x i32> poison, <8 x i32> <i32 1, i32 3, i32 5, i32 7, i32 9, i32 11, i32 13, i32 15>
  %1018 = icmp eq <8 x i32> %strided.vec446, splat (i32 2)
  %1019 = select <8 x i1> %1018, <8 x i32> undef, <8 x i32> %strided.vec447
  %1020 = getelementptr inbounds nuw [8 x i8], ptr %996, i64 %index444
  %interleaved.vec448 = shufflevector <8 x i32> %strided.vec446, <8 x i32> %1019, <16 x i32> <i32 0, i32 8, i32 1, i32 9, i32 2, i32 10, i32 3, i32 11, i32 4, i32 12, i32 5, i32 13, i32 6, i32 14, i32 7, i32 15>
  store <16 x i32> %interleaved.vec448, ptr %1020, align 4, !alias.scope !23651, !noalias !23653
  %index.next449 = add nuw i64 %index444, 8
  %1021 = icmp eq i64 %index.next449, %n.vec442
  br i1 %1021, label %vec.epilog.middle.block450, label %vec.epilog.vector.body443, !llvm.loop !23661

vec.epilog.middle.block450:                       ; preds = %vec.epilog.vector.body443
  %ind.escape451 = add nsw i64 %n.vec442, -1
  %cmp.n452 = icmp eq i64 %979, %n.vec442
  br i1 %cmp.n452, label %.loopexit471, label %vec.epilog.scalar.ph437.preheader

vec.epilog.scalar.ph437.preheader:                ; preds = %vector.memcheck, %iter.check436, %vec.epilog.iter.check438, %vec.epilog.middle.block450
  %.ph = phi i64 [ 0, %iter.check436 ], [ 0, %vector.memcheck ], [ %n.vec424, %vec.epilog.iter.check438 ], [ %n.vec442, %vec.epilog.middle.block450 ]
  %xtraiter563 = and i64 %979, 7
  %lcmp.mod564.not = icmp eq i64 %xtraiter563, 0
  br i1 %lcmp.mod564.not, label %vec.epilog.scalar.ph437.prol.loopexit, label %vec.epilog.scalar.ph437.prol

vec.epilog.scalar.ph437.prol:                     ; preds = %vec.epilog.scalar.ph437.preheader, %vec.epilog.scalar.ph437.prol
  %1022 = phi i64 [ %1031, %vec.epilog.scalar.ph437.prol ], [ %.ph, %vec.epilog.scalar.ph437.preheader ]
  %prol.iter = phi i64 [ %prol.iter.next, %vec.epilog.scalar.ph437.prol ], [ 0, %vec.epilog.scalar.ph437.preheader ]
  %1023 = getelementptr inbounds nuw [8 x i8], ptr %1006, i64 %1022
  %1024 = load i32, ptr %1023, align 4, !range !1785, !noalias !23638, !noundef !1740
  %1025 = getelementptr i8, ptr %1023, i64 4
  %1026 = load i32, ptr %1025, align 4, !noalias !23638
  %1027 = icmp eq i32 %1024, 2
  %1028 = select i1 %1027, i32 undef, i32 %1026
  %1029 = getelementptr inbounds nuw [8 x i8], ptr %996, i64 %1022
  store i32 %1024, ptr %1029, align 4, !noalias !23653
  %1030 = getelementptr inbounds nuw i8, ptr %1029, i64 4
  store i32 %1028, ptr %1030, align 4, !noalias !23653
  %1031 = add nuw nsw i64 %1022, 1
  %prol.iter.next = add i64 %prol.iter, 1
  %prol.iter.cmp.not = icmp eq i64 %prol.iter.next, %xtraiter563
  br i1 %prol.iter.cmp.not, label %vec.epilog.scalar.ph437.prol.loopexit, label %vec.epilog.scalar.ph437.prol, !llvm.loop !23662

vec.epilog.scalar.ph437.prol.loopexit:            ; preds = %vec.epilog.scalar.ph437.prol, %vec.epilog.scalar.ph437.preheader
  %.lcssa482.unr = phi i64 [ poison, %vec.epilog.scalar.ph437.preheader ], [ %1022, %vec.epilog.scalar.ph437.prol ]
  %.unr565 = phi i64 [ %.ph, %vec.epilog.scalar.ph437.preheader ], [ %1031, %vec.epilog.scalar.ph437.prol ]
  %1032 = sub nsw i64 %.ph, %979
  %1033 = icmp ugt i64 %1032, -8
  br i1 %1033, label %.loopexit471, label %vec.epilog.scalar.ph437

vec.epilog.scalar.ph437:                          ; preds = %vec.epilog.scalar.ph437.prol.loopexit, %vec.epilog.scalar.ph437
  %1034 = phi i64 [ %1106, %vec.epilog.scalar.ph437 ], [ %.unr565, %vec.epilog.scalar.ph437.prol.loopexit ]
  %1035 = getelementptr inbounds nuw [8 x i8], ptr %1006, i64 %1034
  %1036 = load i32, ptr %1035, align 4, !range !1785, !noalias !23638, !noundef !1740
  %1037 = getelementptr i8, ptr %1035, i64 4
  %1038 = load i32, ptr %1037, align 4, !noalias !23638
  %1039 = icmp eq i32 %1036, 2
  %1040 = select i1 %1039, i32 undef, i32 %1038
  %1041 = getelementptr inbounds nuw [8 x i8], ptr %996, i64 %1034
  store i32 %1036, ptr %1041, align 4, !noalias !23653
  %1042 = getelementptr inbounds nuw i8, ptr %1041, i64 4
  store i32 %1040, ptr %1042, align 4, !noalias !23653
  %1043 = add nuw nsw i64 %1034, 1
  %1044 = getelementptr inbounds nuw [8 x i8], ptr %1006, i64 %1043
  %1045 = load i32, ptr %1044, align 4, !range !1785, !noalias !23638, !noundef !1740
  %1046 = getelementptr i8, ptr %1044, i64 4
  %1047 = load i32, ptr %1046, align 4, !noalias !23638
  %1048 = icmp eq i32 %1045, 2
  %1049 = select i1 %1048, i32 undef, i32 %1047
  %1050 = getelementptr inbounds nuw [8 x i8], ptr %996, i64 %1043
  store i32 %1045, ptr %1050, align 4, !noalias !23653
  %1051 = getelementptr inbounds nuw i8, ptr %1050, i64 4
  store i32 %1049, ptr %1051, align 4, !noalias !23653
  %1052 = add nuw nsw i64 %1034, 2
  %1053 = getelementptr inbounds nuw [8 x i8], ptr %1006, i64 %1052
  %1054 = load i32, ptr %1053, align 4, !range !1785, !noalias !23638, !noundef !1740
  %1055 = getelementptr i8, ptr %1053, i64 4
  %1056 = load i32, ptr %1055, align 4, !noalias !23638
  %1057 = icmp eq i32 %1054, 2
  %1058 = select i1 %1057, i32 undef, i32 %1056
  %1059 = getelementptr inbounds nuw [8 x i8], ptr %996, i64 %1052
  store i32 %1054, ptr %1059, align 4, !noalias !23653
  %1060 = getelementptr inbounds nuw i8, ptr %1059, i64 4
  store i32 %1058, ptr %1060, align 4, !noalias !23653
  %1061 = add nuw nsw i64 %1034, 3
  %1062 = getelementptr inbounds nuw [8 x i8], ptr %1006, i64 %1061
  %1063 = load i32, ptr %1062, align 4, !range !1785, !noalias !23638, !noundef !1740
  %1064 = getelementptr i8, ptr %1062, i64 4
  %1065 = load i32, ptr %1064, align 4, !noalias !23638
  %1066 = icmp eq i32 %1063, 2
  %1067 = select i1 %1066, i32 undef, i32 %1065
  %1068 = getelementptr inbounds nuw [8 x i8], ptr %996, i64 %1061
  store i32 %1063, ptr %1068, align 4, !noalias !23653
  %1069 = getelementptr inbounds nuw i8, ptr %1068, i64 4
  store i32 %1067, ptr %1069, align 4, !noalias !23653
  %1070 = add nuw nsw i64 %1034, 4
  %1071 = getelementptr inbounds nuw [8 x i8], ptr %1006, i64 %1070
  %1072 = load i32, ptr %1071, align 4, !range !1785, !noalias !23638, !noundef !1740
  %1073 = getelementptr i8, ptr %1071, i64 4
  %1074 = load i32, ptr %1073, align 4, !noalias !23638
  %1075 = icmp eq i32 %1072, 2
  %1076 = select i1 %1075, i32 undef, i32 %1074
  %1077 = getelementptr inbounds nuw [8 x i8], ptr %996, i64 %1070
  store i32 %1072, ptr %1077, align 4, !noalias !23653
  %1078 = getelementptr inbounds nuw i8, ptr %1077, i64 4
  store i32 %1076, ptr %1078, align 4, !noalias !23653
  %1079 = add nuw nsw i64 %1034, 5
  %1080 = getelementptr inbounds nuw [8 x i8], ptr %1006, i64 %1079
  %1081 = load i32, ptr %1080, align 4, !range !1785, !noalias !23638, !noundef !1740
  %1082 = getelementptr i8, ptr %1080, i64 4
  %1083 = load i32, ptr %1082, align 4, !noalias !23638
  %1084 = icmp eq i32 %1081, 2
  %1085 = select i1 %1084, i32 undef, i32 %1083
  %1086 = getelementptr inbounds nuw [8 x i8], ptr %996, i64 %1079
  store i32 %1081, ptr %1086, align 4, !noalias !23653
  %1087 = getelementptr inbounds nuw i8, ptr %1086, i64 4
  store i32 %1085, ptr %1087, align 4, !noalias !23653
  %1088 = add nuw nsw i64 %1034, 6
  %1089 = getelementptr inbounds nuw [8 x i8], ptr %1006, i64 %1088
  %1090 = load i32, ptr %1089, align 4, !range !1785, !noalias !23638, !noundef !1740
  %1091 = getelementptr i8, ptr %1089, i64 4
  %1092 = load i32, ptr %1091, align 4, !noalias !23638
  %1093 = icmp eq i32 %1090, 2
  %1094 = select i1 %1093, i32 undef, i32 %1092
  %1095 = getelementptr inbounds nuw [8 x i8], ptr %996, i64 %1088
  store i32 %1090, ptr %1095, align 4, !noalias !23653
  %1096 = getelementptr inbounds nuw i8, ptr %1095, i64 4
  store i32 %1094, ptr %1096, align 4, !noalias !23653
  %1097 = add nuw nsw i64 %1034, 7
  %1098 = getelementptr inbounds nuw [8 x i8], ptr %1006, i64 %1097
  %1099 = load i32, ptr %1098, align 4, !range !1785, !noalias !23638, !noundef !1740
  %1100 = getelementptr i8, ptr %1098, i64 4
  %1101 = load i32, ptr %1100, align 4, !noalias !23638
  %1102 = icmp eq i32 %1099, 2
  %1103 = select i1 %1102, i32 undef, i32 %1101
  %1104 = getelementptr inbounds nuw [8 x i8], ptr %996, i64 %1097
  store i32 %1099, ptr %1104, align 4, !noalias !23653
  %1105 = getelementptr inbounds nuw i8, ptr %1104, i64 4
  store i32 %1103, ptr %1105, align 4, !noalias !23653
  %1106 = add nuw nsw i64 %1034, 8
  %1107 = icmp eq i64 %1106, %979
  br i1 %1107, label %.loopexit471, label %vec.epilog.scalar.ph437, !llvm.loop !23663

.loopexit471:                                     ; preds = %vec.epilog.scalar.ph437.prol.loopexit, %vec.epilog.scalar.ph437, %vec.epilog.middle.block450, %middle.block433
  %.lcssa = phi i64 [ %ind.escape451, %vec.epilog.middle.block450 ], [ %ind.escape, %middle.block433 ], [ %.lcssa482.unr, %vec.epilog.scalar.ph437.prol.loopexit ], [ %1097, %vec.epilog.scalar.ph437 ]
  %1108 = icmp samesign ult i64 %979, 1152921504606846976
  call void @llvm.assume(i1 %1108), !noalias !23469
  %1109 = add nuw nsw i64 %.lcssa, 2
  %1110 = add nuw nsw i64 %979, 1
  br label %1113

1111:                                             ; preds = %1127, %969, %947
  %1112 = icmp eq ptr %910, %896
  br i1 %1112, label %.loopexit77, label %908

1113:                                             ; preds = %.loopexit471, %998
  %1114 = phi ptr [ %996, %.loopexit471 ], [ %1000, %998 ]
  %1115 = phi i64 [ %1110, %.loopexit471 ], [ %999, %998 ]
  %1116 = phi i64 [ %1109, %.loopexit471 ], [ %1001, %998 ]
  call void @llvm.experimental.noalias.scope.decl(metadata !23664)
  %1117 = load i64, ptr %903, align 8, !alias.scope !23664, !noalias !23667, !noundef !1740
  %1118 = load i64, ptr %39, align 8, !range !1835, !alias.scope !23664, !noalias !23667, !noundef !1740
  %1119 = icmp eq i64 %1117, %1118
  br i1 %1119, label %1120, label %1127

1120:                                             ; preds = %1113
; invoke <alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one
  invoke void @<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %39)
          to label %1127 unwind label %1121, !noalias !23667

1121:                                             ; preds = %1120
  %1122 = landingpad { ptr, i32 }
          cleanup
  %1123 = icmp samesign ugt i64 %1115, 5
  br i1 %1123, label %1124, label %915

1124:                                             ; preds = %1121
  %1125 = shl nuw i64 %1115, 3
  %1126 = add i64 %1125, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %1114, i64 noundef %1126, i64 noundef range(i64 1, -9223372036854775807) 4) #92, !noalias !23669
  br label %915

1127:                                             ; preds = %1120, %1113
  %1128 = load ptr, ptr %904, align 8, !alias.scope !23664, !noalias !23667, !nonnull !1740, !noundef !1740
  %1129 = getelementptr inbounds nuw [40 x i8], ptr %1128, i64 %1117
  store i64 %1115, ptr %1129, align 8, !noalias !23672
  %1130 = getelementptr inbounds nuw i8, ptr %1129, i64 8
  store ptr %1114, ptr %1130, align 8, !noalias !23672
  %1131 = getelementptr inbounds nuw i8, ptr %1129, i64 16
  store i64 %1116, ptr %1131, align 8, !noalias !23672
  %1132 = getelementptr inbounds nuw i8, ptr %1129, i64 24
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %1132, ptr noundef nonnull align 8 dereferenceable(16) %33, i64 16, i1 false), !noalias !23672
  %1133 = add i64 %1117, 1
  store i64 %1133, ptr %903, align 8, !alias.scope !23664, !noalias !23667
  call void @llvm.lifetime.end.p0(ptr nonnull %33)
  br label %1111

1134:                                             ; preds = %959
  call void @llvm.lifetime.end.p0(ptr nonnull %36), !noalias !23464
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %39), !noalias !23469
  call void @llvm.lifetime.end.p0(ptr nonnull %39), !noalias !23464
  call void @llvm.lifetime.end.p0(ptr nonnull %42), !noalias !23464
  br label %1135

1135:                                             ; preds = %1134, %796
  call void @llvm.lifetime.end.p0(ptr nonnull %43), !noalias !23464
  br label %1355

.loopexit77:                                      ; preds = %1111, %917, %893
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(200) %36)
          to label %1136 unwind label %840

1136:                                             ; preds = %.loopexit77
  call void @llvm.lifetime.end.p0(ptr nonnull %36), !noalias !23464
  br label %845

1137:                                             ; preds = %892
  unreachable

1138:                                             ; preds = %851, %848, %845
  call void @llvm.lifetime.end.p0(ptr nonnull %48), !noalias !23464
  br label %538

1139:                                             ; preds = %1204
  %1140 = landingpad { ptr, i32 }
          cleanup
  br label %529

1141:                                             ; preds = %538
  %1142 = getelementptr inbounds nuw i8, ptr %541, i64 16
  %1143 = load i8, ptr %1142, align 8, !noalias !23673
  %1144 = icmp eq i8 %1143, -1
  br i1 %1144, label %1151, label %1145

1145:                                             ; preds = %1141
  %1146 = getelementptr inbounds nuw i8, ptr %541, i64 17
  call void @llvm.lifetime.start.p0(ptr nonnull %25), !noalias !23464
  store i8 %1143, ptr %25, align 8, !noalias !23464
  %1147 = getelementptr inbounds nuw i8, ptr %25, i64 1
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(23) %1147, ptr noundef nonnull align 1 dereferenceable(23) %1146, i64 23, i1 false)
  call void @llvm.lifetime.start.p0(ptr nonnull %24), !noalias !23464
  %1148 = load ptr, ptr %52, align 8, !noalias !23464, !nonnull !1740, !noundef !1740
  %1149 = atomicrmw add ptr %1148, i64 1 monotonic, align 8, !noalias !23469
  %1150 = icmp slt i64 %1149, 0
  br i1 %1150, label %1207, label %1205

1151:                                             ; preds = %1141, %538
  call void @llvm.lifetime.start.p0(ptr nonnull %23), !noalias !23464
  call void @llvm.lifetime.start.p0(ptr nonnull %22), !noalias !23464
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(104) %22, ptr noundef nonnull align 8 dereferenceable(104) %54, i64 104, i1 false), !noalias !23674
  call void @llvm.lifetime.start.p0(ptr nonnull %21), !noalias !23464
  %1152 = load ptr, ptr %52, align 8, !noalias !23464, !nonnull !1740, !noundef !1740
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %21, ptr noundef nonnull align 8 dereferenceable(24) %49, i64 24, i1 false), !noalias !23464
  %1153 = getelementptr inbounds nuw i8, ptr %21, i64 24
  store ptr %1152, ptr %1153, align 8, !noalias !23464
  call void @llvm.experimental.noalias.scope.decl(metadata !23675)
  call void @llvm.experimental.noalias.scope.decl(metadata !23678)
  call void @llvm.experimental.noalias.scope.decl(metadata !23680)
  %1154 = load i64, ptr %22, align 8, !range !2059, !alias.scope !23678, !noalias !23682, !noundef !1740
  %1155 = icmp eq i64 %1154, -1
  br i1 %1155, label %1157, label %1156

1156:                                             ; preds = %1151
; call <purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
  call fastcc void @<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(96) %23, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(32) %21, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(104) %54), !noalias !23683
  br label %1159

1157:                                             ; preds = %1151
  %1158 = getelementptr inbounds nuw i8, ptr %23, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %1158, ptr noundef nonnull readonly align 8 dereferenceable(32) %21, i64 32, i1 false), !alias.scope !23684, !noalias !23685
  store i64 -1, ptr %23, align 8, !alias.scope !23675, !noalias !23686
  br label %1159

1159:                                             ; preds = %1157, %1156
  %1160 = getelementptr inbounds nuw i8, ptr %22, i64 72
  %1161 = load i64, ptr %1160, align 8, !range !1778, !alias.scope !23687, !noalias !23682, !noundef !1740
  %1162 = icmp ugt i64 %1161, 5
  br i1 %1162, label %1163, label %1197

1163:                                             ; preds = %1159
  %1164 = getelementptr inbounds nuw i8, ptr %22, i64 80
  %1165 = load ptr, ptr %1164, align 8, !alias.scope !23678, !noalias !23682, !nonnull !1740, !noundef !1740
  %1166 = mul i64 %1161, 3
  %1167 = add i64 %1166, -3
  %1168 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1169 = load i64, ptr %1168, align 8, !noalias !23690, !noundef !1740
  %1170 = call i64 @llvm.umin.i64(i64 %1167, i64 9223372036854775807)
  %1171 = call i64 @llvm.ssub.sat.i64(i64 %1169, i64 %1170)
  store i64 %1171, ptr %1168, align 8, !noalias !23690
  %1172 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1173 = load i64, ptr %1172, align 8, !noalias !23690, !noundef !1740
  %1174 = icmp slt i64 %1171, %1173
  br i1 %1174, label %1175, label %.preheader479

1175:                                             ; preds = %1163
  store i64 %1171, ptr %1172, align 8, !noalias !23690
  br label %.preheader479

.preheader479:                                    ; preds = %1175, %1163
  br label %1176

1176:                                             ; preds = %.preheader479, %1179
  %1177 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !23690
  %1178 = icmp slt i64 %1177, 0
  br i1 %1178, label %1179, label %__rustc::__rust_dealloc (.exit64)

1179:                                             ; preds = %1176
  %1180 = add nsw i64 %1177, 1
  %1181 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %1177, i64 %1180 acq_rel acquire, align 8, !noalias !23690
  %1182 = extractvalue { i64, i1 } %1181, 1
  br i1 %1182, label %1183, label %1176

1183:                                             ; preds = %1179
  %1184 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1170 monotonic, align 8, !noalias !23690
  %1185 = call i64 @llvm.ssub.sat.i64(i64 %1184, i64 %1170)
  %1186 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !23690
  br label %1187

1187:                                             ; preds = %1190, %1183
  %1188 = phi i64 [ %1186, %1183 ], [ %1193, %1190 ]
  %1189 = icmp slt i64 %1185, %1188
  br i1 %1189, label %1190, label %1194

1190:                                             ; preds = %1187
  %1191 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1188, i64 %1185 monotonic monotonic, align 8, !noalias !23690
  %1192 = extractvalue { i64, i1 } %1191, 1
  %1193 = extractvalue { i64, i1 } %1191, 0
  br i1 %1192, label %1194, label %1187

1194:                                             ; preds = %1190, %1187
  %1195 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !23690
  br label %__rustc::__rust_dealloc (.exit64)

__rustc::__rust_dealloc (.exit64): ; preds = %1176, %1194
  %1196 = icmp ne i64 %1167, 0
  call void @llvm.assume(i1 %1196), !noalias !23690
  call void @free(ptr noundef nonnull %1165) #92, !noalias !23690
  br label %1197

1197:                                             ; preds = %__rustc::__rust_dealloc (.exit64), %1159
  %1198 = getelementptr inbounds nuw i8, ptr %22, i64 96
  call void @llvm.experimental.noalias.scope.decl(metadata !23693), !noalias !23469
  %1199 = load ptr, ptr %1198, align 8, !alias.scope !23696, !noalias !23682, !noundef !1740
  %1200 = icmp eq ptr %1199, null
  br i1 %1200, label %1291, label %1201

1201:                                             ; preds = %1197
  %1202 = atomicrmw sub ptr %1199, i64 1 release, align 8, !noalias !23697
  %1203 = icmp eq i64 %1202, 1
  br i1 %1203, label %1204, label %1291

1204:                                             ; preds = %1201
  fence acquire, !noalias !23469
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %1198) #91
          to label %1291 unwind label %1139

1205:                                             ; preds = %1145
  %1206 = load ptr, ptr %52, align 8, !noalias !23464, !nonnull !1740, !noundef !1740
; invoke <purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::barred_at
  invoke fastcc void @<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::barred_at(ptr noalias nofree noundef align 8 captures(address) dereferenceable(96) %24, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %1, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %25, ptr noundef nonnull %1206)
          to label %1208 unwind label %1353, !noalias !23469

1207:                                             ; preds = %1145
  call void @llvm.trap()
  unreachable

1208:                                             ; preds = %1205
  %1209 = getelementptr inbounds nuw i8, ptr %0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(96) %1209, ptr noundef nonnull align 8 dereferenceable(96) %24, i64 96, i1 false), !noalias !23505
  call void @llvm.lifetime.end.p0(ptr nonnull %24), !noalias !23464
  store i64 0, ptr %0, align 16, !alias.scope !23457, !noalias !23505
  call void @llvm.lifetime.end.p0(ptr nonnull %25), !noalias !23464
  call void @llvm.experimental.noalias.scope.decl(metadata !23702)
  %1210 = getelementptr inbounds nuw i8, ptr %49, i64 8
  %1211 = load ptr, ptr %1210, align 8, !alias.scope !23702, !noalias !23469, !nonnull !1740, !noundef !1740
  %1212 = getelementptr inbounds nuw i8, ptr %49, i64 16
  %1213 = load i64, ptr %1212, align 8, !alias.scope !23702, !noalias !23469, !noundef !1740
  call void @llvm.experimental.noalias.scope.decl(metadata !23705), !noalias !23469
  %1214 = icmp eq i64 %1213, 0
  br i1 %1214, label %.loopexit75, label %.preheader74

.preheader74:                                     ; preds = %1208
  %1215 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1216 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  br label %1217

1217:                                             ; preds = %.preheader74, %1255
  %1218 = phi i64 [ %1220, %1255 ], [ 0, %.preheader74 ]
  %1219 = getelementptr inbounds nuw [40 x i8], ptr %1211, i64 %1218
  %1220 = add nuw nsw i64 %1218, 1
  %1221 = load i64, ptr %1219, align 8, !range !1778, !alias.scope !23708, !noalias !23711, !noundef !1740
  %1222 = icmp ugt i64 %1221, 5
  br i1 %1222, label %1223, label %1255

1223:                                             ; preds = %1217
  %1224 = getelementptr i8, ptr %1219, i64 8
  %1225 = load ptr, ptr %1224, align 8, !alias.scope !23705, !noalias !23711, !nonnull !1740, !noundef !1740
  %1226 = shl i64 %1221, 3
  %1227 = add i64 %1226, -8
  %1228 = load i64, ptr %1215, align 8, !noalias !23712, !noundef !1740
  %1229 = call i64 @llvm.umin.i64(i64 %1227, i64 9223372036854775807)
  %1230 = call i64 @llvm.ssub.sat.i64(i64 %1228, i64 %1229)
  store i64 %1230, ptr %1215, align 8, !noalias !23712
  %1231 = load i64, ptr %1216, align 8, !noalias !23712, !noundef !1740
  %1232 = icmp slt i64 %1230, %1231
  br i1 %1232, label %1233, label %.preheader481

1233:                                             ; preds = %1223
  store i64 %1230, ptr %1216, align 8, !noalias !23712
  br label %.preheader481

.preheader481:                                    ; preds = %1233, %1223
  br label %1234

1234:                                             ; preds = %.preheader481, %1237
  %1235 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !23712
  %1236 = icmp slt i64 %1235, 0
  br i1 %1236, label %1237, label %__rustc::__rust_dealloc (.exit65)

1237:                                             ; preds = %1234
  %1238 = add nsw i64 %1235, 1
  %1239 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %1235, i64 %1238 acq_rel acquire, align 8, !noalias !23712
  %1240 = extractvalue { i64, i1 } %1239, 1
  br i1 %1240, label %1241, label %1234

1241:                                             ; preds = %1237
  %1242 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1229 monotonic, align 8, !noalias !23712
  %1243 = call i64 @llvm.ssub.sat.i64(i64 %1242, i64 %1229)
  %1244 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !23712
  br label %1245

1245:                                             ; preds = %1248, %1241
  %1246 = phi i64 [ %1244, %1241 ], [ %1251, %1248 ]
  %1247 = icmp slt i64 %1243, %1246
  br i1 %1247, label %1248, label %1252

1248:                                             ; preds = %1245
  %1249 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1246, i64 %1243 monotonic monotonic, align 8, !noalias !23712
  %1250 = extractvalue { i64, i1 } %1249, 1
  %1251 = extractvalue { i64, i1 } %1249, 0
  br i1 %1250, label %1252, label %1245

1252:                                             ; preds = %1248, %1245
  %1253 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !23712
  br label %__rustc::__rust_dealloc (.exit65)

__rustc::__rust_dealloc (.exit65): ; preds = %1234, %1252
  %1254 = icmp ne i64 %1227, 0
  call void @llvm.assume(i1 %1254), !noalias !23712
  call void @free(ptr noundef nonnull %1225) #92, !noalias !23712
  br label %1255

1255:                                             ; preds = %__rustc::__rust_dealloc (.exit65), %1217
  %1256 = icmp eq i64 %1220, %1213
  br i1 %1256, label %.loopexit75, label %1217

.loopexit75:                                      ; preds = %1255, %1208
  %1257 = load i64, ptr %49, align 8, !alias.scope !23702, !noalias !23469
  %1258 = icmp eq i64 %1257, 0
  br i1 %1258, label %1289, label %1259

1259:                                             ; preds = %.loopexit75
  %1260 = mul nuw i64 %1257, 40
  %1261 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1262 = load i64, ptr %1261, align 8, !noalias !23711, !noundef !1740
  %1263 = call i64 @llvm.umin.i64(i64 %1260, i64 9223372036854775807)
  %1264 = call i64 @llvm.ssub.sat.i64(i64 %1262, i64 %1263)
  store i64 %1264, ptr %1261, align 8, !noalias !23711
  %1265 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1266 = load i64, ptr %1265, align 8, !noalias !23711, !noundef !1740
  %1267 = icmp slt i64 %1264, %1266
  br i1 %1267, label %1268, label %.preheader480

1268:                                             ; preds = %1259
  store i64 %1264, ptr %1265, align 8, !noalias !23711
  br label %.preheader480

.preheader480:                                    ; preds = %1268, %1259
  br label %1269

1269:                                             ; preds = %.preheader480, %1272
  %1270 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !23711
  %1271 = icmp slt i64 %1270, 0
  br i1 %1271, label %1272, label %__rustc::__rust_dealloc (.exit66)

1272:                                             ; preds = %1269
  %1273 = add nsw i64 %1270, 1
  %1274 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %1270, i64 %1273 acq_rel acquire, align 8, !noalias !23711
  %1275 = extractvalue { i64, i1 } %1274, 1
  br i1 %1275, label %1276, label %1269

1276:                                             ; preds = %1272
  %1277 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1263 monotonic, align 8, !noalias !23711
  %1278 = call i64 @llvm.ssub.sat.i64(i64 %1277, i64 %1263)
  %1279 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !23711
  br label %1280

1280:                                             ; preds = %1283, %1276
  %1281 = phi i64 [ %1279, %1276 ], [ %1286, %1283 ]
  %1282 = icmp slt i64 %1278, %1281
  br i1 %1282, label %1283, label %1287

1283:                                             ; preds = %1280
  %1284 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1281, i64 %1278 monotonic monotonic, align 8, !noalias !23711
  %1285 = extractvalue { i64, i1 } %1284, 1
  %1286 = extractvalue { i64, i1 } %1284, 0
  br i1 %1285, label %1287, label %1280

1287:                                             ; preds = %1283, %1280
  %1288 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !23711
  br label %__rustc::__rust_dealloc (.exit66)

__rustc::__rust_dealloc (.exit66): ; preds = %1269, %1287
  call void @free(ptr noundef nonnull %1211) #92, !noalias !23711
  br label %1289

1289:                                             ; preds = %1366, %__rustc::__rust_dealloc (.exit66), %.loopexit75, %535
  %1290 = phi i8 [ 1, %1366 ], [ 0, %535 ], [ %539, %.loopexit75 ], [ %539, %__rustc::__rust_dealloc (.exit66) ]
  call void @llvm.lifetime.end.p0(ptr nonnull %49), !noalias !23464
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>(ptr noalias nofree noundef align 8 dereferenceable(216) %50)
          to label %1367 unwind label %196, !noalias !23469

1291:                                             ; preds = %1204, %1201, %1197
  call void @llvm.lifetime.end.p0(ptr nonnull %21), !noalias !23464
  call void @llvm.lifetime.end.p0(ptr nonnull %22), !noalias !23464
  %1292 = getelementptr inbounds nuw i8, ptr %0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(96) %1292, ptr noundef nonnull align 8 dereferenceable(96) %23, i64 96, i1 false), !noalias !23505
  store i64 0, ptr %0, align 16, !alias.scope !23457, !noalias !23505
  call void @llvm.lifetime.end.p0(ptr nonnull %23), !noalias !23464
  call void @llvm.lifetime.end.p0(ptr nonnull %49), !noalias !23464
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>(ptr noalias nofree noundef align 8 dereferenceable(216) %50)
          to label %1293 unwind label %196, !noalias !23469

1293:                                             ; preds = %1291
  call void @llvm.lifetime.end.p0(ptr nonnull %50), !noalias !23464
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(200) %51)
          to label %1294 unwind label %162

1294:                                             ; preds = %1293
  call void @llvm.lifetime.end.p0(ptr nonnull %51), !noalias !23464
  call void @llvm.lifetime.end.p0(ptr nonnull %52), !noalias !23464
  call void @llvm.experimental.noalias.scope.decl(metadata !23715)
  call void @llvm.experimental.noalias.scope.decl(metadata !23718), !noalias !23721
  %1295 = load ptr, ptr %151, align 8, !alias.scope !23722, !noalias !23721, !nonnull !1740, !noundef !1740
  %1296 = atomicrmw sub ptr %1295, i64 1 release, align 8, !noalias !23723
  %1297 = icmp eq i64 %1296, 1
  br i1 %1297, label %1298, label %1302

1298:                                             ; preds = %1294
  fence acquire, !noalias !23721
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %151) #91
          to label %1302 unwind label %1299

1299:                                             ; preds = %1298
  %1300 = landingpad { ptr, i32 }
          cleanup
  %1301 = trunc nuw i8 %539 to i1
  br i1 %1301, label %1572, label %1574

1302:                                             ; preds = %1298, %1294
  %1303 = trunc nuw i8 %539 to i1
  br i1 %1303, label %1304, label %1570

1304:                                             ; preds = %1302
  call void @llvm.experimental.noalias.scope.decl(metadata !23724)
  %1305 = getelementptr inbounds nuw i8, ptr %57, i64 8
  %1306 = load ptr, ptr %1305, align 8, !alias.scope !23724, !nonnull !1740, !noundef !1740
  %1307 = load i64, ptr %187, align 8, !alias.scope !23724, !noundef !1740
  call void @llvm.experimental.noalias.scope.decl(metadata !23727)
  %1308 = icmp eq i64 %1307, 0
  br i1 %1308, label %.loopexit73, label %.preheader72

.preheader72:                                     ; preds = %1304
  %1309 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1310 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  br label %1311

1311:                                             ; preds = %.preheader72, %1349
  %1312 = phi i64 [ %1314, %1349 ], [ 0, %.preheader72 ]
  %1313 = getelementptr inbounds nuw [40 x i8], ptr %1306, i64 %1312
  %1314 = add nuw nsw i64 %1312, 1
  %1315 = load i64, ptr %1313, align 8, !range !1778, !alias.scope !23730, !noalias !23724, !noundef !1740
  %1316 = icmp ugt i64 %1315, 5
  br i1 %1316, label %1317, label %1349

1317:                                             ; preds = %1311
  %1318 = getelementptr i8, ptr %1313, i64 8
  %1319 = load ptr, ptr %1318, align 8, !alias.scope !23727, !noalias !23724, !nonnull !1740, !noundef !1740
  %1320 = shl i64 %1315, 3
  %1321 = add i64 %1320, -8
  %1322 = load i64, ptr %1309, align 8, !noalias !23733, !noundef !1740
  %1323 = call i64 @llvm.umin.i64(i64 %1321, i64 9223372036854775807)
  %1324 = call i64 @llvm.ssub.sat.i64(i64 %1322, i64 %1323)
  store i64 %1324, ptr %1309, align 8, !noalias !23733
  %1325 = load i64, ptr %1310, align 8, !noalias !23733, !noundef !1740
  %1326 = icmp slt i64 %1324, %1325
  br i1 %1326, label %1327, label %.preheader478

1327:                                             ; preds = %1317
  store i64 %1324, ptr %1310, align 8, !noalias !23733
  br label %.preheader478

.preheader478:                                    ; preds = %1327, %1317
  br label %1328

1328:                                             ; preds = %.preheader478, %1331
  %1329 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !23733
  %1330 = icmp slt i64 %1329, 0
  br i1 %1330, label %1331, label %__rustc::__rust_dealloc (.exit67)

1331:                                             ; preds = %1328
  %1332 = add nsw i64 %1329, 1
  %1333 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %1329, i64 %1332 acq_rel acquire, align 8, !noalias !23733
  %1334 = extractvalue { i64, i1 } %1333, 1
  br i1 %1334, label %1335, label %1328

1335:                                             ; preds = %1331
  %1336 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1323 monotonic, align 8, !noalias !23733
  %1337 = call i64 @llvm.ssub.sat.i64(i64 %1336, i64 %1323)
  %1338 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !23733
  br label %1339

1339:                                             ; preds = %1342, %1335
  %1340 = phi i64 [ %1338, %1335 ], [ %1345, %1342 ]
  %1341 = icmp slt i64 %1337, %1340
  br i1 %1341, label %1342, label %1346

1342:                                             ; preds = %1339
  %1343 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1340, i64 %1337 monotonic monotonic, align 8, !noalias !23733
  %1344 = extractvalue { i64, i1 } %1343, 1
  %1345 = extractvalue { i64, i1 } %1343, 0
  br i1 %1344, label %1346, label %1339

1346:                                             ; preds = %1342, %1339
  %1347 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !23733
  br label %__rustc::__rust_dealloc (.exit67)

__rustc::__rust_dealloc (.exit67): ; preds = %1328, %1346
  %1348 = icmp ne i64 %1321, 0
  call void @llvm.assume(i1 %1348), !noalias !23733
  call void @free(ptr noundef nonnull %1319) #92, !noalias !23733
  br label %1349

1349:                                             ; preds = %__rustc::__rust_dealloc (.exit67), %1311
  %1350 = icmp eq i64 %1314, %1307
  br i1 %1350, label %.loopexit73, label %1311

.loopexit73:                                      ; preds = %1349, %1304
  %1351 = load i64, ptr %57, align 8, !alias.scope !23724
  %1352 = icmp eq i64 %1351, 0
  br i1 %1352, label %1570, label %1538

1353:                                             ; preds = %1205
  %1354 = landingpad { ptr, i32 }
          cleanup
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %49) #89, !noalias !23469
  br label %529

1355:                                             ; preds = %1135, %596
  call void @llvm.experimental.noalias.scope.decl(metadata !23736)
  %1356 = load ptr, ptr %48, align 8, !alias.scope !23736, !noalias !23469, !noundef !1740
  %1357 = icmp eq ptr %1356, null
  br i1 %1357, label %1366, label %1358

1358:                                             ; preds = %1355
  %1359 = atomicrmw sub ptr %1356, i64 1 release, align 8, !noalias !23739
  %1360 = icmp eq i64 %1359, 1
  br i1 %1360, label %1361, label %1366

1361:                                             ; preds = %1358
  fence acquire, !noalias !23469
; invoke <alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %48) #91
          to label %1366 unwind label %533, !inline_history !2025

1362:                                             ; preds = %794, %788, %772, %635
  %1363 = phi { ptr, i32 } [ %636, %635 ], [ %795, %794 ], [ %789, %788 ], [ %773, %772 ]
; invoke core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>>(ptr noalias nofree noundef align 8 dereferenceable(32) %43) #89
          to label %571 unwind label %550, !noalias !23469

1364:                                             ; preds = %602
  %1365 = landingpad { ptr, i32 }
          cleanup
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %14) #89
  br label %571

1366:                                             ; preds = %1361, %1358, %1355
  call void @llvm.lifetime.end.p0(ptr nonnull %48), !noalias !23464
  br label %1289

1367:                                             ; preds = %1289
  call void @llvm.lifetime.end.p0(ptr nonnull %50), !noalias !23464
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(200) %51)
          to label %1368 unwind label %159

1368:                                             ; preds = %1367
  call void @llvm.lifetime.end.p0(ptr nonnull %51), !noalias !23464
  call void @llvm.experimental.noalias.scope.decl(metadata !23742)
  call void @llvm.experimental.noalias.scope.decl(metadata !23745), !noalias !23469
  %1369 = load ptr, ptr %52, align 8, !alias.scope !23748, !noalias !23469, !nonnull !1740, !noundef !1740
  %1370 = atomicrmw sub ptr %1369, i64 1 release, align 8, !noalias !23749
  %1371 = icmp eq i64 %1370, 1
  br i1 %1371, label %1372, label %1375

1372:                                             ; preds = %1368
  fence acquire, !noalias !23469
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %52) #91
          to label %1375 unwind label %1373

1373:                                             ; preds = %1372
  %1374 = landingpad { ptr, i32 }
          cleanup
  br label %1531

1375:                                             ; preds = %1372, %1368
  call void @llvm.lifetime.end.p0(ptr nonnull %52), !noalias !23464
  call void @llvm.experimental.noalias.scope.decl(metadata !23750)
  %1376 = getelementptr inbounds nuw i8, ptr %54, i64 72
  %1377 = load i64, ptr %1376, align 8, !range !1778, !alias.scope !23753, !noalias !23683, !noundef !1740
  %1378 = icmp ugt i64 %1377, 5
  br i1 %1378, label %1379, label %1413

1379:                                             ; preds = %1375
  %1380 = getelementptr inbounds nuw i8, ptr %54, i64 80
  %1381 = load ptr, ptr %1380, align 8, !alias.scope !23750, !noalias !23683, !nonnull !1740, !noundef !1740
  %1382 = mul i64 %1377, 3
  %1383 = add i64 %1382, -3
  %1384 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1385 = load i64, ptr %1384, align 8, !noalias !23756, !noundef !1740
  %1386 = call i64 @llvm.umin.i64(i64 %1383, i64 9223372036854775807)
  %1387 = call i64 @llvm.ssub.sat.i64(i64 %1385, i64 %1386)
  store i64 %1387, ptr %1384, align 8, !noalias !23756
  %1388 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1389 = load i64, ptr %1388, align 8, !noalias !23756, !noundef !1740
  %1390 = icmp slt i64 %1387, %1389
  br i1 %1390, label %1391, label %.preheader477

1391:                                             ; preds = %1379
  store i64 %1387, ptr %1388, align 8, !noalias !23756
  br label %.preheader477

.preheader477:                                    ; preds = %1391, %1379
  br label %1392

1392:                                             ; preds = %.preheader477, %1395
  %1393 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !23756
  %1394 = icmp slt i64 %1393, 0
  br i1 %1394, label %1395, label %__rustc::__rust_dealloc (.exit68)

1395:                                             ; preds = %1392
  %1396 = add nsw i64 %1393, 1
  %1397 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %1393, i64 %1396 acq_rel acquire, align 8, !noalias !23756
  %1398 = extractvalue { i64, i1 } %1397, 1
  br i1 %1398, label %1399, label %1392

1399:                                             ; preds = %1395
  %1400 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1386 monotonic, align 8, !noalias !23756
  %1401 = call i64 @llvm.ssub.sat.i64(i64 %1400, i64 %1386)
  %1402 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !23756
  br label %1403

1403:                                             ; preds = %1406, %1399
  %1404 = phi i64 [ %1402, %1399 ], [ %1409, %1406 ]
  %1405 = icmp slt i64 %1401, %1404
  br i1 %1405, label %1406, label %1410

1406:                                             ; preds = %1403
  %1407 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1404, i64 %1401 monotonic monotonic, align 8, !noalias !23756
  %1408 = extractvalue { i64, i1 } %1407, 1
  %1409 = extractvalue { i64, i1 } %1407, 0
  br i1 %1408, label %1410, label %1403

1410:                                             ; preds = %1406, %1403
  %1411 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !23756
  br label %__rustc::__rust_dealloc (.exit68)

__rustc::__rust_dealloc (.exit68): ; preds = %1392, %1410
  %1412 = icmp ne i64 %1383, 0
  call void @llvm.assume(i1 %1412), !noalias !23756
  call void @free(ptr noundef nonnull %1381) #92, !noalias !23756
  br label %1413

1413:                                             ; preds = %__rustc::__rust_dealloc (.exit68), %1375
  %1414 = load i64, ptr %54, align 8, !range !2059, !alias.scope !23750, !noalias !23683, !noundef !1740
  %1415 = icmp sgt i64 %1414, 0
  br i1 %1415, label %1416, label %1448

1416:                                             ; preds = %1413
  %1417 = getelementptr inbounds nuw i8, ptr %54, i64 8
  %1418 = load ptr, ptr %1417, align 8, !alias.scope !23750, !noalias !23683, !nonnull !1740, !noundef !1740
  %1419 = mul nuw i64 %1414, 3
  %1420 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1421 = load i64, ptr %1420, align 8, !noalias !23759, !noundef !1740
  %1422 = call i64 @llvm.umin.i64(i64 %1419, i64 9223372036854775807)
  %1423 = call i64 @llvm.ssub.sat.i64(i64 %1421, i64 %1422)
  store i64 %1423, ptr %1420, align 8, !noalias !23759
  %1424 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1425 = load i64, ptr %1424, align 8, !noalias !23759, !noundef !1740
  %1426 = icmp slt i64 %1423, %1425
  br i1 %1426, label %1427, label %.preheader476

1427:                                             ; preds = %1416
  store i64 %1423, ptr %1424, align 8, !noalias !23759
  br label %.preheader476

.preheader476:                                    ; preds = %1427, %1416
  br label %1428

1428:                                             ; preds = %.preheader476, %1431
  %1429 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !23759
  %1430 = icmp slt i64 %1429, 0
  br i1 %1430, label %1431, label %__rustc::__rust_dealloc (.exit69)

1431:                                             ; preds = %1428
  %1432 = add nsw i64 %1429, 1
  %1433 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %1429, i64 %1432 acq_rel acquire, align 8, !noalias !23759
  %1434 = extractvalue { i64, i1 } %1433, 1
  br i1 %1434, label %1435, label %1428

1435:                                             ; preds = %1431
  %1436 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1422 monotonic, align 8, !noalias !23759
  %1437 = call i64 @llvm.ssub.sat.i64(i64 %1436, i64 %1422)
  %1438 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !23759
  br label %1439

1439:                                             ; preds = %1442, %1435
  %1440 = phi i64 [ %1438, %1435 ], [ %1445, %1442 ]
  %1441 = icmp slt i64 %1437, %1440
  br i1 %1441, label %1442, label %1446

1442:                                             ; preds = %1439
  %1443 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1440, i64 %1437 monotonic monotonic, align 8, !noalias !23759
  %1444 = extractvalue { i64, i1 } %1443, 1
  %1445 = extractvalue { i64, i1 } %1443, 0
  br i1 %1444, label %1446, label %1439

1446:                                             ; preds = %1442, %1439
  %1447 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !23759
  br label %__rustc::__rust_dealloc (.exit69)

__rustc::__rust_dealloc (.exit69): ; preds = %1428, %1446
  call void @free(ptr noundef nonnull %1418) #92, !noalias !23759
  br label %1448

1448:                                             ; preds = %__rustc::__rust_dealloc (.exit69), %1413
  %1449 = getelementptr inbounds nuw i8, ptr %54, i64 96
  call void @llvm.experimental.noalias.scope.decl(metadata !23760), !noalias !23683
  %1450 = load ptr, ptr %1449, align 8, !alias.scope !23763, !noalias !23683, !noundef !1740
  %1451 = icmp eq ptr %1450, null
  br i1 %1451, label %1465, label %1452

1452:                                             ; preds = %1448
  %1453 = atomicrmw sub ptr %1450, i64 1 release, align 8, !noalias !23764
  %1454 = icmp eq i64 %1453, 1
  br i1 %1454, label %1455, label %1465

1455:                                             ; preds = %1452
  fence acquire, !noalias !23683
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %1449) #91
          to label %1465 unwind label %1463

1456:                                             ; preds = %1531, %1463, %162, %158
  %1457 = phi i8 [ %1290, %1463 ], [ %1532, %1531 ], [ %193, %158 ], [ %539, %162 ]
  %1458 = phi { ptr, i32 } [ %1464, %1463 ], [ %1533, %1531 ], [ %195, %158 ], [ %163, %162 ]
  call void @llvm.experimental.noalias.scope.decl(metadata !23769)
  call void @llvm.experimental.noalias.scope.decl(metadata !23772), !noalias !23457
  %1459 = load ptr, ptr %151, align 8, !alias.scope !23775, !noalias !23457, !nonnull !1740, !noundef !1740
  %1460 = atomicrmw sub ptr %1459, i64 1 release, align 8, !noalias !23776
  %1461 = icmp eq i64 %1460, 1
  br i1 %1461, label %1462, label %1534

1462:                                             ; preds = %1456
  fence acquire, !noalias !23457
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %151) #91
          to label %1534 unwind label %550

1463:                                             ; preds = %1455
  %1464 = landingpad { ptr, i32 }
          cleanup
  br label %1456

1465:                                             ; preds = %1455, %1452, %1448
  call void @llvm.experimental.noalias.scope.decl(metadata !23777)
  call void @llvm.experimental.noalias.scope.decl(metadata !23780), !noalias !23457
  %1466 = load ptr, ptr %151, align 8, !alias.scope !23783, !noalias !23457, !nonnull !1740, !noundef !1740
  %1467 = atomicrmw sub ptr %1466, i64 1 release, align 8, !noalias !23784
  %1468 = icmp eq i64 %1467, 1
  br i1 %1468, label %1469, label %1473

1469:                                             ; preds = %1465
  fence acquire, !noalias !23457
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %151) #91
          to label %1473 unwind label %1470

1470:                                             ; preds = %1469
  %1471 = landingpad { ptr, i32 }
          cleanup
  %1472 = trunc nuw i8 %1290 to i1
  br i1 %1472, label %1572, label %1574

1473:                                             ; preds = %1469, %1465
  %1474 = trunc nuw i8 %1290 to i1
  br i1 %1474, label %1475, label %1570

1475:                                             ; preds = %1473
  call void @llvm.experimental.noalias.scope.decl(metadata !23785)
  %1476 = getelementptr inbounds nuw i8, ptr %57, i64 8
  %1477 = load ptr, ptr %1476, align 8, !alias.scope !23785, !nonnull !1740, !noundef !1740
  %1478 = load i64, ptr %187, align 8, !alias.scope !23785, !noundef !1740
  call void @llvm.experimental.noalias.scope.decl(metadata !23788)
  %1479 = icmp eq i64 %1478, 0
  br i1 %1479, label %.loopexit, label %.preheader

.preheader:                                       ; preds = %1475
  %1480 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1481 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  br label %1482

1482:                                             ; preds = %.preheader, %1520
  %1483 = phi i64 [ %1485, %1520 ], [ 0, %.preheader ]
  %1484 = getelementptr inbounds nuw [40 x i8], ptr %1477, i64 %1483
  %1485 = add nuw nsw i64 %1483, 1
  %1486 = load i64, ptr %1484, align 8, !range !1778, !alias.scope !23791, !noalias !23785, !noundef !1740
  %1487 = icmp ugt i64 %1486, 5
  br i1 %1487, label %1488, label %1520

1488:                                             ; preds = %1482
  %1489 = getelementptr i8, ptr %1484, i64 8
  %1490 = load ptr, ptr %1489, align 8, !alias.scope !23788, !noalias !23785, !nonnull !1740, !noundef !1740
  %1491 = shl i64 %1486, 3
  %1492 = add i64 %1491, -8
  %1493 = load i64, ptr %1480, align 8, !noalias !23794, !noundef !1740
  %1494 = call i64 @llvm.umin.i64(i64 %1492, i64 9223372036854775807)
  %1495 = call i64 @llvm.ssub.sat.i64(i64 %1493, i64 %1494)
  store i64 %1495, ptr %1480, align 8, !noalias !23794
  %1496 = load i64, ptr %1481, align 8, !noalias !23794, !noundef !1740
  %1497 = icmp slt i64 %1495, %1496
  br i1 %1497, label %1498, label %.preheader475

1498:                                             ; preds = %1488
  store i64 %1495, ptr %1481, align 8, !noalias !23794
  br label %.preheader475

.preheader475:                                    ; preds = %1498, %1488
  br label %1499

1499:                                             ; preds = %.preheader475, %1502
  %1500 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !23794
  %1501 = icmp slt i64 %1500, 0
  br i1 %1501, label %1502, label %__rustc::__rust_dealloc (.exit70)

1502:                                             ; preds = %1499
  %1503 = add nsw i64 %1500, 1
  %1504 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %1500, i64 %1503 acq_rel acquire, align 8, !noalias !23794
  %1505 = extractvalue { i64, i1 } %1504, 1
  br i1 %1505, label %1506, label %1499

1506:                                             ; preds = %1502
  %1507 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1494 monotonic, align 8, !noalias !23794
  %1508 = call i64 @llvm.ssub.sat.i64(i64 %1507, i64 %1494)
  %1509 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !23794
  br label %1510

1510:                                             ; preds = %1513, %1506
  %1511 = phi i64 [ %1509, %1506 ], [ %1516, %1513 ]
  %1512 = icmp slt i64 %1508, %1511
  br i1 %1512, label %1513, label %1517

1513:                                             ; preds = %1510
  %1514 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1511, i64 %1508 monotonic monotonic, align 8, !noalias !23794
  %1515 = extractvalue { i64, i1 } %1514, 1
  %1516 = extractvalue { i64, i1 } %1514, 0
  br i1 %1515, label %1517, label %1510

1517:                                             ; preds = %1513, %1510
  %1518 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !23794
  br label %__rustc::__rust_dealloc (.exit70)

__rustc::__rust_dealloc (.exit70): ; preds = %1499, %1517
  %1519 = icmp ne i64 %1492, 0
  call void @llvm.assume(i1 %1519), !noalias !23794
  call void @free(ptr noundef nonnull %1490) #92, !noalias !23794
  br label %1520

1520:                                             ; preds = %__rustc::__rust_dealloc (.exit70), %1482
  %1521 = icmp eq i64 %1485, %1478
  br i1 %1521, label %.loopexit, label %1482

.loopexit:                                        ; preds = %1520, %1475
  %1522 = load i64, ptr %57, align 8, !alias.scope !23785
  %1523 = icmp eq i64 %1522, 0
  br i1 %1523, label %1570, label %1538

1524:                                             ; preds = %159, %158
  %1525 = phi { ptr, i32 } [ %161, %159 ], [ %195, %158 ]
  %1526 = phi i8 [ %160, %159 ], [ %193, %158 ]
  call void @llvm.experimental.noalias.scope.decl(metadata !23797)
  call void @llvm.experimental.noalias.scope.decl(metadata !23800), !noalias !23469
  %1527 = load ptr, ptr %52, align 8, !alias.scope !23803, !noalias !23469, !nonnull !1740, !noundef !1740
  %1528 = atomicrmw sub ptr %1527, i64 1 release, align 8, !noalias !23804
  %1529 = icmp eq i64 %1528, 1
  br i1 %1529, label %1530, label %1531

1530:                                             ; preds = %1524
  fence acquire, !noalias !23469
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %52) #91
          to label %1531 unwind label %550

1531:                                             ; preds = %1530, %1524, %1373
  %1532 = phi i8 [ %1290, %1373 ], [ %1526, %1530 ], [ %1526, %1524 ]
  %1533 = phi { ptr, i32 } [ %1374, %1373 ], [ %1525, %1530 ], [ %1525, %1524 ]
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>(ptr noalias nofree noundef nonnull align 8 dereferenceable(104) %54) #89
          to label %1456 unwind label %550, !noalias !23683

1534:                                             ; preds = %1462, %1456
  %1535 = trunc nuw i8 %1457 to i1
  br i1 %1535, label %1572, label %1574

1536:                                             ; preds = %147
  call void @llvm.lifetime.end.p0(ptr nonnull %55)
  call void @llvm.lifetime.end.p0(ptr nonnull %56)
  %1537 = getelementptr inbounds nuw i8, ptr %0, i64 8
; call <purrdf_sparql_eval::governor::lift::Lift>::withheld::<purrdf_core::ir::term::TermId>
  call fastcc void @<purrdf_sparql_eval::governor::lift::Lift>::withheld::<purrdf_core::ir::term::TermId>(ptr noalias nofree noundef align 8 captures(address) dereferenceable(96) %1537, ptr noalias nofree noundef align 8 captures(address) dereferenceable(104) %58)
  store i64 0, ptr %0, align 16
  br label %1571

1538:                                             ; preds = %.loopexit, %.loopexit73
  %1539 = phi i64 [ %1351, %.loopexit73 ], [ %1522, %.loopexit ]
  %1540 = phi ptr [ %1306, %.loopexit73 ], [ %1477, %.loopexit ]
  %1541 = mul nuw i64 %1539, 40
  %1542 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1543 = load i64, ptr %1542, align 8, !noalias !1740, !noundef !1740
  %1544 = call i64 @llvm.umin.i64(i64 %1541, i64 9223372036854775807)
  %1545 = call i64 @llvm.ssub.sat.i64(i64 %1543, i64 %1544)
  store i64 %1545, ptr %1542, align 8, !noalias !1740
  %1546 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1547 = load i64, ptr %1546, align 8, !noalias !1740, !noundef !1740
  %1548 = icmp slt i64 %1545, %1547
  br i1 %1548, label %1549, label %.preheader474

1549:                                             ; preds = %1538
  store i64 %1545, ptr %1546, align 8, !noalias !1740
  br label %.preheader474

.preheader474:                                    ; preds = %1549, %1538
  br label %1550

1550:                                             ; preds = %.preheader474, %1553
  %1551 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !1740
  %1552 = icmp slt i64 %1551, 0
  br i1 %1552, label %1553, label %__rustc::__rust_dealloc (.exit71)

1553:                                             ; preds = %1550
  %1554 = add nsw i64 %1551, 1
  %1555 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %1551, i64 %1554 acq_rel acquire, align 8, !noalias !1740
  %1556 = extractvalue { i64, i1 } %1555, 1
  br i1 %1556, label %1557, label %1550

1557:                                             ; preds = %1553
  %1558 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1544 monotonic, align 8, !noalias !1740
  %1559 = call i64 @llvm.ssub.sat.i64(i64 %1558, i64 %1544)
  %1560 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !1740
  br label %1561

1561:                                             ; preds = %1564, %1557
  %1562 = phi i64 [ %1560, %1557 ], [ %1567, %1564 ]
  %1563 = icmp slt i64 %1559, %1562
  br i1 %1563, label %1564, label %1568

1564:                                             ; preds = %1561
  %1565 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1562, i64 %1559 monotonic monotonic, align 8, !noalias !1740
  %1566 = extractvalue { i64, i1 } %1565, 1
  %1567 = extractvalue { i64, i1 } %1565, 0
  br i1 %1566, label %1568, label %1561

1568:                                             ; preds = %1564, %1561
  %1569 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !1740
  br label %__rustc::__rust_dealloc (.exit71)

__rustc::__rust_dealloc (.exit71): ; preds = %1550, %1568
  call void @free(ptr noundef nonnull %1540) #92, !noalias !1740
  br label %1570

1570:                                             ; preds = %__rustc::__rust_dealloc (.exit71), %.loopexit, %1473, %.loopexit73, %1302
  call void @llvm.lifetime.end.p0(ptr nonnull %54)
  call void @llvm.lifetime.end.p0(ptr nonnull %57)
  br label %1571

1571:                                             ; preds = %1570, %1536, %144, %141, %137
  ret void

1572:                                             ; preds = %1534, %1470, %1299
  %1573 = phi { ptr, i32 } [ %1300, %1299 ], [ %1471, %1470 ], [ %1458, %1534 ]
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef nonnull align 8 dereferenceable(32) %57) #89, !noalias !23457
  br label %1574

1574:                                             ; preds = %1576, %1572, %1534, %1470, %1299
  %1575 = phi { ptr, i32 } [ %1300, %1299 ], [ %1577, %1576 ], [ %1458, %1534 ], [ %1471, %1470 ], [ %1573, %1572 ]
  resume { ptr, i32 } %1575

1576:                                             ; preds = %145, %5
  %1577 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>(ptr noalias nofree noundef align 8 dereferenceable(104) %58) #89
          to label %1574 unwind label %1578

1578:                                             ; preds = %1576
  %1579 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  tail call void @core::panicking::panic_in_cleanup() #90
  unreachable
}

define internal fastcc void @<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>> as alloc::vec::spec_from_iter::SpecFromIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, core::iter::adapters::map::Map<core::slice::iter::Iter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>, purrdf_sparql_eval::modifier::eval_project_sequence<purrdf_core::ir::term::TermId>::{closure#1}>>>::from_iter(ptr dead_on_unwind noalias nofree noundef nonnull writable writeonly align 8 captures(none) dereferenceable(24) %0, ptr noalias nofree noundef nonnull readonly align 8 captures(none) dead_on_return dereferenceable(24) %1) unnamed_addr #0 personality ptr @rust_eh_personality !guid !115163 {
  %3 = alloca [8 x i8], align 8
  %4 = alloca [40 x i8], align 8
  %5 = alloca [40 x i8], align 8
  %6 = alloca [24 x i8], align 8
  tail call void @llvm.experimental.noalias.scope.decl(metadata !115164)
  call void @llvm.lifetime.start.p0(ptr nonnull %6), !noalias !115167
  %7 = load ptr, ptr %1, align 8, !alias.scope !115164, !noalias !115169, !nonnull !1733, !noundef !1733
  %8 = getelementptr inbounds nuw i8, ptr %1, i64 8
  %9 = load ptr, ptr %8, align 8, !alias.scope !115164, !noalias !115169, !nonnull !1733, !noundef !1733
  %10 = ptrtoint ptr %9 to i64
  %11 = ptrtoint ptr %7 to i64
  %12 = sub nuw i64 %10, %11
  %13 = udiv exact i64 %12, 40
  %14 = icmp ugt i64 %12, 9223372036854775800
  br i1 %14, label %__rustc::__rust_alloc (.exit.thread), label %15, !prof !6969

15:                                               ; preds = %2
  %16 = icmp eq ptr %9, %7
  br i1 %16, label %17, label %20

17:                                               ; preds = %15
  store i64 0, ptr %6, align 8, !noalias !115167
  %18 = getelementptr inbounds nuw i8, ptr %6, i64 8
  store ptr inttoptr (i64 8 to ptr), ptr %18, align 8, !noalias !115167
  %19 = getelementptr inbounds nuw i8, ptr %6, i64 16
  br label %.loopexit20

20:                                               ; preds = %15
  %21 = icmp samesign ugt i64 %12, 7
  br i1 %21, label %26, label %22

22:                                               ; preds = %20
  call void @llvm.lifetime.start.p0(ptr nonnull %3), !noalias !115170
  store ptr null, ptr %3, align 8, !noalias !115170
  %23 = call noundef i32 @posix_memalign(ptr noundef nonnull %3, i64 noundef 8, i64 noundef range(i64 1, 0) %12) #92, !noalias !115170
  %24 = icmp eq i32 %23, 0
  %25 = load ptr, ptr %3, align 8, !noalias !115170
  call void @llvm.lifetime.end.p0(ptr nonnull %3), !noalias !115170
  br i1 %24, label %28, label %__rustc::__rust_alloc (.exit.thread)

26:                                               ; preds = %20
  %27 = tail call noundef ptr @malloc(i64 noundef range(i64 1, 0) %12) #92, !noalias !115170
  br label %28

28:                                               ; preds = %26, %22
  %29 = phi ptr [ %27, %26 ], [ %25, %22 ]
  %30 = icmp eq ptr %29, null
  br i1 %30, label %__rustc::__rust_alloc (.exit.thread), label %31

31:                                               ; preds = %28
  %32 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %33 = load i64, ptr %32, align 8, !noalias !115170, !noundef !1733
  %34 = call i64 @llvm.uadd.sat.i64(i64 %33, i64 1)
  store i64 %34, ptr %32, align 8, !noalias !115170
  %35 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %36 = load i64, ptr %35, align 8, !noalias !115170, !noundef !1733
  %37 = call i64 @llvm.uadd.sat.i64(i64 %36, i64 %12)
  store i64 %37, ptr %35, align 8, !noalias !115170
  %38 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %39 = load i64, ptr %38, align 8, !noalias !115170, !noundef !1733
  %40 = call i64 @llvm.sadd.sat.i64(i64 %39, i64 %12)
  store i64 %40, ptr %38, align 8, !noalias !115170
  %41 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %42 = load i64, ptr %41, align 8, !noalias !115170, !noundef !1733
  %43 = icmp sgt i64 %40, %42
  br i1 %43, label %44, label %.preheader

44:                                               ; preds = %31
  store i64 %40, ptr %41, align 8, !noalias !115170
  br label %.preheader

.preheader:                                       ; preds = %44, %31
  br label %45

45:                                               ; preds = %.preheader, %48
  %46 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !115170
  %47 = icmp slt i64 %46, 0
  br i1 %47, label %48, label %__rustc::__rust_alloc (.exit)

48:                                               ; preds = %45
  %49 = add nsw i64 %46, 1
  %50 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %46, i64 %49 acq_rel acquire, align 8, !noalias !115170
  %51 = extractvalue { i64, i1 } %50, 1
  br i1 %51, label %52, label %45

52:                                               ; preds = %48
  %53 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_ALLOCATIONS, i64 1 monotonic, align 8, !noalias !115170
  %54 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_REQUESTED_BYTES, i64 %12 monotonic, align 8, !noalias !115170
  %55 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %12 monotonic, align 8, !noalias !115170
  %56 = call i64 @llvm.sadd.sat.i64(i64 %55, i64 %12)
  %57 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES monotonic, align 8, !noalias !115170
  br label %58

58:                                               ; preds = %61, %52
  %59 = phi i64 [ %57, %52 ], [ %64, %61 ]
  %60 = icmp sgt i64 %56, %59
  br i1 %60, label %61, label %65

61:                                               ; preds = %58
  %62 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES, i64 %59, i64 %56 monotonic monotonic, align 8, !noalias !115170
  %63 = extractvalue { i64, i1 } %62, 1
  %64 = extractvalue { i64, i1 } %62, 0
  br i1 %63, label %65, label %58

65:                                               ; preds = %61, %58
  %66 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !115170
  br label %__rustc::__rust_alloc (.exit)

__rustc::__rust_alloc (.exit.thread): ; preds = %22, %28, %2
  %67 = phi i64 [ 8, %22 ], [ 0, %2 ], [ 8, %28 ]
; call alloc::raw_vec::handle_error
  tail call void @alloc::raw_vec::handle_error(i64 noundef %67, i64 %12) #93, !noalias !115167
  unreachable

__rustc::__rust_alloc (.exit):  ; preds = %45, %65
  store i64 %13, ptr %6, align 8, !noalias !115167
  %68 = getelementptr inbounds nuw i8, ptr %6, i64 8
  store ptr %29, ptr %68, align 8, !noalias !115167
  %69 = getelementptr inbounds nuw i8, ptr %6, i64 16
  %70 = getelementptr inbounds nuw i8, ptr %1, i64 16
  %71 = load ptr, ptr %70, align 8, !alias.scope !115164, !noalias !115169, !nonnull !1733, !noundef !1733
  tail call void @llvm.experimental.noalias.scope.decl(metadata !115173)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !115176)
  %72 = getelementptr inbounds nuw i8, ptr %71, i64 8
  %73 = getelementptr inbounds nuw i8, ptr %71, i64 16
  %74 = getelementptr inbounds nuw i8, ptr %4, i64 16
  %75 = getelementptr inbounds nuw i8, ptr %4, i64 8
  br label %76

76:                                               ; preds = %.loopexit, %__rustc::__rust_alloc (.exit)
  %77 = phi i64 [ 0, %__rustc::__rust_alloc (.exit) ], [ %210, %.loopexit ]
  %78 = getelementptr inbounds nuw [40 x i8], ptr %7, i64 %77
  call void @llvm.experimental.noalias.scope.decl(metadata !115179)
  call void @llvm.lifetime.start.p0(ptr nonnull %5)
  call void @llvm.experimental.noalias.scope.decl(metadata !115182)
  %79 = load ptr, ptr %72, align 8, !noalias !115185, !nonnull !1733, !noundef !1733
  %80 = load i64, ptr %73, align 8, !noalias !115185, !noundef !1733
  %81 = getelementptr inbounds nuw [16 x i8], ptr %79, i64 %80
  call void @llvm.lifetime.start.p0(ptr nonnull %4), !noalias !115198
  store i64 1, ptr %4, align 8, !noalias !115198
  %82 = icmp ugt i64 %80, 4
  br i1 %82, label %83, label %94, !prof !1735

83:                                               ; preds = %76
; invoke <purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow
  invoke void @<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow(ptr noalias nofree noundef nonnull align 8 dereferenceable(40) %4, i64 noundef 0, i64 noundef %80, i1 noundef zeroext true) #91
          to label %84 unwind label %197, !noalias !115198

84:                                               ; preds = %83
  %85 = load i64, ptr %4, align 8, !range !1771, !alias.scope !115202, !noalias !115205
  %86 = freeze i64 %85
  %87 = add i64 %86, -1
  %88 = call i64 @llvm.umax.i64(i64 %87, i64 4)
  %89 = icmp ugt i64 %87, 4
  %90 = load ptr, ptr %75, align 8, !alias.scope !115202, !noalias !115205
  %91 = select i1 %89, ptr %90, ptr %75
  %92 = select i1 %89, ptr %74, ptr %4
  %93 = load i64, ptr %92, align 8, !alias.scope !115202, !noalias !115205
  br label %94

94:                                               ; preds = %84, %76
  %95 = phi i64 [ %93, %84 ], [ 1, %76 ]
  %96 = phi ptr [ %91, %84 ], [ %75, %76 ]
  %97 = phi i64 [ %88, %84 ], [ 4, %76 ]
  %98 = phi ptr [ %92, %84 ], [ %4, %76 ]
  %99 = add i64 %95, -1
  %100 = icmp ult i64 %99, %97
  br i1 %100, label %101, label %114

101:                                              ; preds = %94
  %102 = getelementptr inbounds nuw i8, ptr %78, i64 8
  %103 = getelementptr inbounds nuw i8, ptr %78, i64 16
  %104 = load i64, ptr %78, align 8, !range !1771, !alias.scope !115207, !noalias !115208
  %105 = add i64 %104, -1
  %106 = icmp ugt i64 %105, 4
  %107 = load i64, ptr %103, align 8, !alias.scope !115207, !noalias !115208
  %108 = add i64 %107, -1
  %109 = select i1 %106, i64 %108, i64 %105
  %110 = load ptr, ptr %102, align 8, !alias.scope !115207, !noalias !115208, !nonnull !1733
  %111 = select i1 %106, ptr %110, ptr %102
  br label %129

112:                                              ; preds = %185
  %113 = add nuw i64 %97, 1
  br label %114

114:                                              ; preds = %112, %94
  %115 = phi ptr [ %79, %94 ], [ %134, %112 ]
  %116 = phi i64 [ %95, %94 ], [ %113, %112 ]
  store i64 %116, ptr %98, align 8, !alias.scope !115202, !noalias !115205
  %117 = icmp eq ptr %115, %81
  br i1 %117, label %.loopexit, label %118

118:                                              ; preds = %114
  %119 = getelementptr inbounds nuw i8, ptr %78, i64 8
  %120 = getelementptr inbounds nuw i8, ptr %78, i64 16
  %121 = load i64, ptr %78, align 8, !range !1771, !alias.scope !115207, !noalias !115208
  %122 = add i64 %121, -1
  %123 = icmp ugt i64 %122, 4
  %124 = load i64, ptr %120, align 8, !alias.scope !115207, !noalias !115208
  %125 = add i64 %124, -1
  %126 = select i1 %123, i64 %125, i64 %122
  %127 = load ptr, ptr %119, align 8, !alias.scope !115207, !noalias !115208, !nonnull !1733
  %128 = select i1 %123, ptr %127, ptr %119
  br label %146

129:                                              ; preds = %185, %101
  %130 = phi i64 [ %99, %101 ], [ %188, %185 ]
  %131 = phi ptr [ %79, %101 ], [ %134, %185 ]
  %132 = icmp eq ptr %131, %81
  br i1 %132, label %190, label %133

133:                                              ; preds = %129
  %134 = getelementptr inbounds nuw i8, ptr %131, i64 16
  %135 = load i64, ptr %131, align 8, !range !1732, !noalias !115209, !noundef !1733
  %136 = getelementptr i8, ptr %131, i64 8
  %137 = load i64, ptr %136, align 8, !noalias !115209
  %138 = trunc nuw i64 %135 to i1
  br i1 %138, label %139, label %185

139:                                              ; preds = %133
  %140 = icmp ult i64 %137, %109
  br i1 %140, label %143, label %141

141:                                              ; preds = %139
; invoke core::panicking::panic_bounds_check
  invoke void @core::panicking::panic_bounds_check(i64 noundef %137, i64 noundef %109, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.68dd637f94a7f528fe69f6876e3d956b.792) #88
          to label %142 unwind label %192, !noalias !115205

142:                                              ; preds = %141
  unreachable

143:                                              ; preds = %139
  %144 = getelementptr inbounds nuw [8 x i8], ptr %111, i64 %137
  %145 = load <2 x i32>, ptr %144, align 4, !noalias !115212
  br label %185

146:                                              ; preds = %179, %118
  %147 = phi ptr [ %115, %118 ], [ %148, %179 ]
  %148 = getelementptr inbounds nuw i8, ptr %147, i64 16
  %149 = load i64, ptr %147, align 8, !range !1732, !noalias !115213, !noundef !1733
  %150 = getelementptr i8, ptr %147, i64 8
  %151 = load i64, ptr %150, align 8, !noalias !115213
  %152 = trunc nuw i64 %149 to i1
  br i1 %152, label %153, label %160

153:                                              ; preds = %146
  %154 = icmp ult i64 %151, %126
  br i1 %154, label %157, label %155

155:                                              ; preds = %153
; invoke core::panicking::panic_bounds_check
  invoke void @core::panicking::panic_bounds_check(i64 noundef %151, i64 noundef %126, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.68dd637f94a7f528fe69f6876e3d956b.792) #88
          to label %156 unwind label %199, !noalias !115198

156:                                              ; preds = %155
  unreachable

157:                                              ; preds = %153
  %158 = getelementptr inbounds nuw [8 x i8], ptr %128, i64 %151
  %159 = load <2 x i32>, ptr %158, align 4, !noalias !115216
  br label %160

160:                                              ; preds = %157, %146
  %161 = phi <2 x i32> [ <i32 2, i32 undef>, %146 ], [ %159, %157 ]
  %162 = load i64, ptr %4, align 8, !range !1771, !alias.scope !115217, !noalias !115205, !noundef !1733
  %163 = add i64 %162, -1
  %164 = icmp ugt i64 %163, 4
  %165 = load ptr, ptr %75, align 8, !alias.scope !115217, !noalias !115205, !nonnull !1733
  %166 = select i1 %164, ptr %165, ptr %75
  %167 = select i1 %164, ptr %74, ptr %4
  %168 = call i64 @llvm.umax.i64(i64 %163, i64 4)
  %169 = load i64, ptr %167, align 8, !alias.scope !115217, !noalias !115205, !noundef !1733
  %170 = add i64 %169, -1
  %171 = icmp eq i64 %170, %168
  br i1 %171, label %172, label %179, !prof !1735

172:                                              ; preds = %160
; invoke <purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow
  invoke void @<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow(ptr noalias nofree noundef nonnull align 8 dereferenceable(40) %4, i64 noundef %168, i64 noundef 1, i1 noundef zeroext true) #91
          to label %173 unwind label %195, !noalias !115198

173:                                              ; preds = %172
  %174 = load i64, ptr %4, align 8, !range !1771, !alias.scope !115217, !noalias !115205, !noundef !1733
  %175 = icmp ugt i64 %174, 5
  %176 = load ptr, ptr %75, align 8, !alias.scope !115217, !noalias !115205, !nonnull !1733
  %177 = select i1 %175, ptr %176, ptr %75
  %178 = select i1 %175, ptr %74, ptr %4
  br label %179

179:                                              ; preds = %173, %160
  %180 = phi ptr [ %177, %173 ], [ %166, %160 ]
  %181 = phi ptr [ %178, %173 ], [ %167, %160 ]
  %182 = getelementptr inbounds nuw [8 x i8], ptr %180, i64 %170
  store <2 x i32> %161, ptr %182, align 4, !noalias !115205
  %183 = add i64 %169, 1
  store i64 %183, ptr %181, align 8, !alias.scope !115217, !noalias !115205
  %184 = icmp eq ptr %148, %81
  br i1 %184, label %.loopexit, label %146

185:                                              ; preds = %143, %133
  %186 = phi <2 x i32> [ <i32 2, i32 undef>, %133 ], [ %145, %143 ]
  %187 = getelementptr inbounds nuw [8 x i8], ptr %96, i64 %130
  store <2 x i32> %186, ptr %187, align 4, !noalias !115205
  %188 = add i64 %130, 1
  %189 = icmp eq i64 %188, %97
  br i1 %189, label %112, label %129

190:                                              ; preds = %129
  %191 = add nuw i64 %130, 1
  store i64 %191, ptr %98, align 8, !alias.scope !115202, !noalias !115205
  br label %.loopexit

192:                                              ; preds = %141
  %193 = landingpad { ptr, i32 }
          cleanup
  %194 = add nuw i64 %130, 1
  store i64 %194, ptr %98, align 8, !alias.scope !115202, !noalias !115205
  br label %201

195:                                              ; preds = %172
  %196 = landingpad { ptr, i32 }
          cleanup
  br label %201

197:                                              ; preds = %83
  %198 = landingpad { ptr, i32 }
          cleanup
  br label %201

199:                                              ; preds = %155
  %200 = landingpad { ptr, i32 }
          cleanup
  br label %201

201:                                              ; preds = %199, %197, %195, %192
  %202 = phi { ptr, i32 } [ %193, %192 ], [ %196, %195 ], [ %198, %197 ], [ %200, %199 ]
  %203 = load i64, ptr %4, align 8, !range !1771, !alias.scope !115220, !noalias !115198, !noundef !1733
  %204 = icmp ugt i64 %203, 5
  br i1 %204, label %205, label %212

205:                                              ; preds = %201
  %206 = load ptr, ptr %75, align 8, !noalias !115198, !nonnull !1733, !noundef !1733
  %207 = shl i64 %203, 3
  %208 = add i64 %207, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %206, i64 noundef %208, i64 noundef range(i64 1, -9223372036854775807) 4) #92, !noalias !115223
  br label %212

.loopexit:                                        ; preds = %179, %190, %114
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %5, ptr noundef nonnull align 8 dereferenceable(40) %4, i64 40, i1 false), !noalias !115226
  call void @llvm.lifetime.end.p0(ptr nonnull %4), !noalias !115198
  %209 = getelementptr inbounds nuw [40 x i8], ptr %29, i64 %77
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %209, ptr noundef nonnull readonly align 8 dereferenceable(40) %5, i64 40, i1 false), !noalias !115227
  %210 = add i64 %77, 1
  call void @llvm.lifetime.end.p0(ptr nonnull %5)
  %211 = icmp eq i64 %210, %13
  br i1 %211, label %.loopexit20, label %76

212:                                              ; preds = %205, %201
  store i64 %77, ptr %69, align 8, !alias.scope !115232, !noalias !115233
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %6) #89, !noalias !115167
  resume { ptr, i32 } %202

.loopexit20:                                      ; preds = %.loopexit, %17
  %213 = phi ptr [ %19, %17 ], [ %69, %.loopexit ]
  %214 = phi i64 [ 0, %17 ], [ %13, %.loopexit ]
  store i64 %214, ptr %213, align 8, !alias.scope !115232, !noalias !115233
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %0, ptr noundef nonnull align 8 dereferenceable(24) %6, i64 24, i1 false), !noalias !115164
  call void @llvm.lifetime.end.p0(ptr nonnull %6), !noalias !115167
  ret void
}

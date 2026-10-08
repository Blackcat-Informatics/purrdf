define internal fastcc void @purrdf_native::py_store::query::materialize_results(ptr dead_on_unwind noalias nofree noundef nonnull writable writeonly align 8 captures(none) dereferenceable(56) %_0, ptr noalias nofree noundef nonnull align 8 captures(address) dead_on_return dereferenceable(56) %result) unnamed_addr #1 personality ptr @rust_eh_personality !guid !160771 {
start:
  %e.i.i.i.i.i.i.i.i.i.i.i.i.i = alloca [0 x i8], align 1
  %_3.i = alloca [56 x i8], align 8
  %_5.i.i.i = alloca [56 x i8], align 8
  %_3.i.i.i = alloca [56 x i8], align 8
  %_3.i6.i = alloca [56 x i8], align 8
  %_3.i.i = alloca [56 x i8], align 8
  %_7.sroa.4.i.i.i.i.i.i = alloca [192 x i8], align 8
  %_34.i.i.i.i.i = alloca [456 x i8], align 8
  %dst_guard.i.i = alloca [24 x i8], align 8
  %_19.i = alloca [32 x i8], align 8
  %_14.i16 = alloca [32 x i8], align 8
  %_10.i = alloca [32 x i8], align 8
  %quads.i = alloca [24 x i8], align 8
  %formatter.i.i.i.i.i.i.i.i.i.i.i.i = alloca [24 x i8], align 8
  %buf.i.i.i.i.i.i.i.i.i.i.i.i = alloca [24 x i8], align 8
  %_2.i3.i.i.i.i.i.i.i.i.i = alloca [80 x i8], align 8
  %_5.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = alloca [80 x i8], align 8
  %_5.sroa.6.i.i.i.i.i.i.i.i.i.i.i.i.sroa.0.i.i.i.i.i.i.i.i.i = alloca [16 x i8], align 8
  %_5.sroa.9.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = alloca [24 x i8], align 8
  %dst_guard.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = alloca [24 x i8], align 8
  %_2.i.i.i.i.i.i.i.i.i.i.i.i.i.i = alloca [40 x i8], align 16
  %value.i.i.i.i.i.i.i.i.i.i.i.i = alloca [24 x i8], align 8
  %residual.i.i.i.i.i.i.i.i.i.i.i.i = alloca [56 x i8], align 8
  %_7.sroa.5.i.i.i.i.i.i.i.i.i.i = alloca [48 x i8], align 8
  %dst_guard.i.i.i.i.i = alloca [24 x i8], align 8
  %_2.i.i.i = alloca [40 x i8], align 8
  %residual.i = alloca [56 x i8], align 8
  %graph = alloca [8 x i8], align 8
  %_15 = alloca [48 x i8], align 8
  %_7.sroa.12 = alloca [24 x i8], align 8
  %rows = alloca [24 x i8], align 8
  %variables = alloca [24 x i8], align 8
  %0 = load i64, ptr %result, align 8, !range !20940, !noundef !3995
  %1 = icmp slt i64 %0, 0
  %2 = add i64 %0, -9223372036854775807
  %_3 = select i1 %1, i64 %2, i64 0
  switch i64 %_3, label %bb1 [
    i64 0, label %bb4
    i64 1, label %bb3
    i64 2, label %bb2
  ]

bb1:                                              ; preds = %start
  unreachable

bb4:                                              ; preds = %start
  call void @llvm.lifetime.start.p0(ptr nonnull %variables)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %variables, ptr noundef nonnull align 8 dereferenceable(24) %result, i64 24, i1 false)
  %3 = getelementptr inbounds nuw i8, ptr %result, i64 24
  %4 = getelementptr inbounds nuw i8, ptr %result, i64 32
  %rows3 = load ptr, ptr %4, align 8, !nonnull !3995, !noundef !3995
  %rows4 = load i64, ptr %3, align 8, !range !4210, !noundef !3995
  %5 = getelementptr inbounds nuw i8, ptr %result, i64 40
  %rows5 = load i64, ptr %5, align 8, !noundef !3995
  call void @llvm.lifetime.start.p0(ptr nonnull %rows)
  call void @llvm.lifetime.start.p0(ptr nonnull %_7.sroa.12)
  %_40 = icmp ult i64 %rows5, 384307168202282326
  tail call void @llvm.assume(i1 %_40)
  %_38.idx = mul nuw nsw i64 %rows5, 24
  %_38 = getelementptr inbounds nuw i8, ptr %rows3, i64 %_38.idx
  call void @llvm.lifetime.start.p0(ptr nonnull %residual.i), !noalias !160772
  store i64 0, ptr %residual.i, align 8, !noalias !160772
  call void @llvm.lifetime.start.p0(ptr nonnull %_2.i.i.i), !noalias !160776
  store ptr %rows3, ptr %_2.i.i.i, align 8, !alias.scope !160783, !noalias !160787
  %_8.sroa.4.0._2.i.i.i.sroa_idx = getelementptr inbounds nuw i8, ptr %_2.i.i.i, i64 8
  %_8.sroa.5.0._2.i.i.i.sroa_idx = getelementptr inbounds nuw i8, ptr %_2.i.i.i, i64 16
  store i64 %rows4, ptr %_8.sroa.5.0._2.i.i.i.sroa_idx, align 8, !alias.scope !160783, !noalias !160787
  %_8.sroa.6.0._2.i.i.i.sroa_idx = getelementptr inbounds nuw i8, ptr %_2.i.i.i, i64 24
  store ptr %_38, ptr %_8.sroa.6.0._2.i.i.i.sroa_idx, align 8, !alias.scope !160783, !noalias !160787
  %_8.sroa.4.0._2.i.i.sroa_idx.i = getelementptr inbounds nuw i8, ptr %_2.i.i.i, i64 32
  store ptr %residual.i, ptr %_8.sroa.4.0._2.i.i.sroa_idx.i, align 8, !alias.scope !160783, !noalias !160788
  call void @llvm.experimental.noalias.scope.decl(metadata !160789)
  call void @llvm.experimental.noalias.scope.decl(metadata !160792)
  call void @llvm.experimental.noalias.scope.decl(metadata !160795)
  call void @llvm.experimental.noalias.scope.decl(metadata !160798)
  call void @llvm.experimental.noalias.scope.decl(metadata !160801)
  call void @llvm.experimental.noalias.scope.decl(metadata !160804)
  %_44.not131.i.i.i.i.i.i.i.i.i = icmp eq i64 %rows5, 0
  br i1 %_44.not131.i.i.i.i.i.i.i.i.i, label %bb6.i.i.i.i.i, label %bb11.lr.ph.i.i.i.i.i.i.i.i.i

bb14.i.i.i.i.i:                                   ; preds = %cleanup3.body.i.i.i.i.i, %bb4.i.i.i.i.i.i.i.i.i.i, %bb11.i.i.i.i.i.i.i.i.i.i.i.body
  %.pn.i.i.i.i.i = phi { ptr, i32 } [ %94, %cleanup3.body.i.i.i.i.i ], [ %eh.lpad-body80, %bb11.i.i.i.i.i.i.i.i.i.i.i.body ], [ %eh.lpad-body.i.i.i218.i.i.i.i.i.i.i.i.i, %bb4.i.i.i.i.i.i.i.i.i.i ]
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<alloc::vec::Vec<core::option::Option<purrdf_core::ir::term::TermValue>>>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<alloc::vec::Vec<core::option::Option<purrdf_core::ir::term::TermValue>>>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(40) %_2.i.i.i)
          to label %bb15.i unwind label %terminate.i.i.i.i.i, !noalias !160807

bb11.lr.ph.i.i.i.i.i.i.i.i.i:                     ; preds = %bb4
  %_3.sroa.4.0._2.i.i.i.sroa_idx.i.i.i.i.i.i.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %_2.i.i.i.i.i.i.i.i.i.i.i.i.i.i, i64 8
  %_3.sroa.5.0._2.i.i.i.sroa_idx.i.i.i.i.i.i.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %_2.i.i.i.i.i.i.i.i.i.i.i.i.i.i, i64 16
  %_3.sroa.6.0._2.i.i.i.sroa_idx.i.i.i.i.i.i.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %_2.i.i.i.i.i.i.i.i.i.i.i.i.i.i, i64 24
  %_8.sroa.4.0._2.i.i.sroa_idx.i.i.i.i.i.i.i.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %_2.i.i.i.i.i.i.i.i.i.i.i.i.i.i, i64 32
  %_8.sroa.5.0._5.i.sroa_idx.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %_5.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, i64 8
  %_6.i.i.i.i.i.i.i.i.i.i.i.i.i.sroa.7.0._2.i3.sroa_idx.i.i.i.i.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %_2.i3.i.i.i.i.i.i.i.i.i, i64 8
  %_6.i.i.i.i.i.i.i.i.i.i.i.i.i.sroa.9.0._2.i3.sroa_idx.i.i.i.i.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %_2.i3.i.i.i.i.i.i.i.i.i, i64 24
  %_6.i.i.i.i.i.i.i.i.i.i.i.i.i.sroa.10.0._2.i3.sroa_idx.i.i.i.i.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %_2.i3.i.i.i.i.i.i.i.i.i, i64 32
  %_6.i.i.i.i.i.i.i.i.i.i.i.i.i.sroa.12.0._2.i3.sroa_idx.i.i.i.i.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %_2.i3.i.i.i.i.i.i.i.i.i, i64 48
  %_6.i.i.i.i.i.i.i.i.i.i.i.i.i.sroa.13.0._2.i3.sroa_idx.i.i.i.i.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %_2.i3.i.i.i.i.i.i.i.i.i, i64 52
  %_6.i.i.i.i.i.i.i.i.i.i.i.i.i.sroa.13.sroa.6.0._6.i.i.i.i.i.i.i.i.i.i.i.i.i.sroa.13.0._2.i3.sroa_idx.sroa_idx.i.i.i.i.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %_2.i3.i.i.i.i.i.i.i.i.i, i64 56
  %_10.sroa.4.0.buf.sroa_idx.i.i.i.i.i.i.i.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %buf.i.i.i.i.i.i.i.i.i.i.i.i, i64 8
  %_10.sroa.5.0.buf.sroa_idx.i.i.i.i.i.i.i.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %buf.i.i.i.i.i.i.i.i.i.i.i.i, i64 16
  %6 = getelementptr inbounds nuw i8, ptr %formatter.i.i.i.i.i.i.i.i.i.i.i.i, i64 16
  %7 = getelementptr inbounds nuw i8, ptr %formatter.i.i.i.i.i.i.i.i.i.i.i.i, i64 8
  %8 = getelementptr inbounds nuw i8, ptr %residual.i.i.i.i.i.i.i.i.i.i.i.i, i64 24
  %9 = getelementptr inbounds nuw i8, ptr %residual.i.i.i.i.i.i.i.i.i.i.i.i, i64 32
  %10 = getelementptr inbounds nuw i8, ptr %residual.i.i.i.i.i.i.i.i.i.i.i.i, i64 40
  %_3.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @pyo3::internal::state::ATTACH_COUNT::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL)
  %_11.sroa.5.0._16.sroa_idx.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %residual.i.i.i.i.i.i.i.i.i.i.i.i, i64 8
  %_5.sroa.6.i.i.i.i.i.i.i.i.i.i.i.i.sroa.14.0._11.sroa.5.0._16.sroa_idx.i.i.i.i.i.i.i.i.i.i.i.i.i.sroa_idx.i.i.i.i.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %residual.i.i.i.i.i.i.i.i.i.i.i.i, i64 48
  %_5.sroa.6.i.i.i.i.i.i.i.i.i.i.i.i.sroa.16.0._11.sroa.5.0._16.sroa_idx.i.i.i.i.i.i.i.i.i.i.i.i.i.sroa_idx.i.i.i.i.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %residual.i.i.i.i.i.i.i.i.i.i.i.i, i64 52
  %11 = getelementptr inbounds nuw i8, ptr %dst_guard.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, i64 8
  %12 = getelementptr inbounds nuw i8, ptr %dst_guard.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, i64 16
  %13 = getelementptr inbounds nuw i8, ptr %value.i.i.i.i.i.i.i.i.i.i.i.i, i64 8
  %14 = getelementptr inbounds nuw i8, ptr %value.i.i.i.i.i.i.i.i.i.i.i.i, i64 16
  br label %bb11.i.i.i.i.i.i.i.i.i

bb11.i.i.i.i.i.i.i.i.i:                           ; preds = %bb14.i.i.i.i.i.i.i.i.i, %bb11.lr.ph.i.i.i.i.i.i.i.i.i
  %_46139.i.i.i.i.i.i.i.i.i = phi ptr [ %rows3, %bb11.lr.ph.i.i.i.i.i.i.i.i.i ], [ %_49.i.i.i.i.i.i.i.i.i, %bb14.i.i.i.i.i.i.i.i.i ]
  %_5.sroa.6.i.i.i.i.i.i.i.i.i.i.i.i.sroa.8.0133.i.i.i.i.i.i.i.i.i = phi i64 [ undef, %bb11.lr.ph.i.i.i.i.i.i.i.i.i ], [ %_5.sroa.6.i.i.i.i.i.i.i.i.i.i.i.i.sroa.8.4.i.i.i.i.i.i.i.i.i, %bb14.i.i.i.i.i.i.i.i.i ]
  %_6.i.i.i.i.i.i.i.i.i.i.i.i.i.sroa.13.sroa.0.0132.i.i.i.i.i.i.i.i.i = phi i32 [ undef, %bb11.lr.ph.i.i.i.i.i.i.i.i.i ], [ %_6.i.i.i.i.i.i.i.i.i.i.i.i.i.sroa.13.sroa.0.4.i.i.i.i.i.i.i.i.i, %bb14.i.i.i.i.i.i.i.i.i ]
  %15 = phi <2 x i32> [ undef, %bb11.lr.ph.i.i.i.i.i.i.i.i.i ], [ %46, %bb14.i.i.i.i.i.i.i.i.i ]
  %16 = phi <2 x ptr> [ undef, %bb11.lr.ph.i.i.i.i.i.i.i.i.i ], [ %47, %bb14.i.i.i.i.i.i.i.i.i ]
  %tmp.sroa.0.0.copyload.i.i.i.i.i.i.i.i.i = load i64, ptr %_46139.i.i.i.i.i.i.i.i.i, align 8, !noalias !160810
  %tmp.sroa.5.0.self1.sroa_idx.i.i.i.i.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %_46139.i.i.i.i.i.i.i.i.i, i64 8
  %tmp.sroa.5.0.copyload.i.i.i.i.i.i.i.i.i = load ptr, ptr %tmp.sroa.5.0.self1.sroa_idx.i.i.i.i.i.i.i.i.i, align 8, !noalias !160810, !nonnull !3995, !noundef !3995
  %tmp.sroa.6.0.self1.sroa_idx.i.i.i.i.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %_46139.i.i.i.i.i.i.i.i.i, i64 16
  %tmp.sroa.6.0.copyload.i.i.i.i.i.i.i.i.i = load i64, ptr %tmp.sroa.6.0.self1.sroa_idx.i.i.i.i.i.i.i.i.i, align 8, !noalias !160810
  %_49.i.i.i.i.i.i.i.i.i = getelementptr i8, ptr %_46139.i.i.i.i.i.i.i.i.i, i64 24
  call void @llvm.lifetime.start.p0(ptr nonnull %_7.sroa.5.i.i.i.i.i.i.i.i.i.i)
  %_12.i.i.i.i.i.i.i.i.i.i.i = icmp ult i64 %tmp.sroa.6.0.copyload.i.i.i.i.i.i.i.i.i, 115292150460684698
  call void @llvm.assume(i1 %_12.i.i.i.i.i.i.i.i.i.i.i)
  %_8.idx.i.i.i.i.i.i.i.i.i.i.i = mul nuw nsw i64 %tmp.sroa.6.0.copyload.i.i.i.i.i.i.i.i.i, 80
  %_8.i.i.i.i.i.i.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %tmp.sroa.5.0.copyload.i.i.i.i.i.i.i.i.i, i64 %_8.idx.i.i.i.i.i.i.i.i.i.i.i
  call void @llvm.lifetime.start.p0(ptr nonnull %residual.i.i.i.i.i.i.i.i.i.i.i.i), !noalias !160813
  store i64 0, ptr %residual.i.i.i.i.i.i.i.i.i.i.i.i, align 8, !noalias !160813
  call void @llvm.lifetime.start.p0(ptr nonnull %value.i.i.i.i.i.i.i.i.i.i.i.i), !noalias !160813
  call void @llvm.experimental.noalias.scope.decl(metadata !160823)
  call void @llvm.experimental.noalias.scope.decl(metadata !160826)
  call void @llvm.lifetime.start.p0(ptr nonnull %_2.i.i.i.i.i.i.i.i.i.i.i.i.i.i), !noalias !160829
  store ptr %tmp.sroa.5.0.copyload.i.i.i.i.i.i.i.i.i, ptr %_2.i.i.i.i.i.i.i.i.i.i.i.i.i.i, align 16, !alias.scope !160832, !noalias !160836
  store i64 %tmp.sroa.0.0.copyload.i.i.i.i.i.i.i.i.i, ptr %_3.sroa.5.0._2.i.i.i.sroa_idx.i.i.i.i.i.i.i.i.i.i.i, align 16, !alias.scope !160832, !noalias !160836
  store ptr %_8.i.i.i.i.i.i.i.i.i.i.i, ptr %_3.sroa.6.0._2.i.i.i.sroa_idx.i.i.i.i.i.i.i.i.i.i.i, align 8, !alias.scope !160832, !noalias !160836
  store ptr %residual.i.i.i.i.i.i.i.i.i.i.i.i, ptr %_8.sroa.4.0._2.i.i.sroa_idx.i.i.i.i.i.i.i.i.i.i.i.i, align 16, !alias.scope !160832, !noalias !160837
  call void @llvm.experimental.noalias.scope.decl(metadata !160838)
  call void @llvm.experimental.noalias.scope.decl(metadata !160841)
  call void @llvm.experimental.noalias.scope.decl(metadata !160843)
  call void @llvm.experimental.noalias.scope.decl(metadata !160846)
  call void @llvm.experimental.noalias.scope.decl(metadata !160848)
  call void @llvm.experimental.noalias.scope.decl(metadata !160851)
  call void @llvm.experimental.noalias.scope.decl(metadata !160854)
  call void @llvm.experimental.noalias.scope.decl(metadata !160857)
  %_44.not9.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = icmp eq i64 %tmp.sroa.6.0.copyload.i.i.i.i.i.i.i.i.i, 0
  br i1 %_44.not9.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, label %bb6.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, label %bb11.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i

bb14.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i:             ; preds = %cleanup3.body.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, %bb4.i.i.i.i.i.i.i.i.i.i.i.i.body.i.i.i.i.i.i.i.i.i, %bb11.i.i.i.i.i.i.i.i.i.i.i.body.i.i.i.i.i.i.i.i.i.i.i
  %.pn.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = phi { ptr, i32 } [ %58, %cleanup3.body.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i ], [ %eh.lpad-body.i.i.i.i.i.i.i.i.i.i.i, %bb11.i.i.i.i.i.i.i.i.i.i.i.body.i.i.i.i.i.i.i.i.i.i.i ], [ %eh.lpad-body.i.i.i.i.i.i.i.i.i, %bb4.i.i.i.i.i.i.i.i.i.i.i.i.body.i.i.i.i.i.i.i.i.i ]
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<core::option::Option<purrdf_core::ir::term::TermValue>>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<core::option::Option<purrdf_core::ir::term::TermValue>>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(40) %_2.i.i.i.i.i.i.i.i.i.i.i.i.i.i)
          to label %bb15.i.i.i.i.i.i.i.i.i.i.i.i unwind label %terminate.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, !noalias !160860

bb11.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i:     ; preds = %bb11.i.i.i.i.i.i.i.i.i, %bb14.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i
  %_6.i.i.i.i.i.i.i.i.i.i.i.i.i.sroa.13.sroa.0.1.i.i.i.i.i.i.i.i.i = phi i32 [ %_6.i.i.i.i.i.i.i.i.i.i.i.i.i.sroa.13.sroa.0.3.i.i.i.i.i.i.i.i.i, %bb14.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i ], [ %_6.i.i.i.i.i.i.i.i.i.i.i.i.i.sroa.13.sroa.0.0132.i.i.i.i.i.i.i.i.i, %bb11.i.i.i.i.i.i.i.i.i ]
  %_5.sroa.6.i.i.i.i.i.i.i.i.i.i.i.i.sroa.8.1.i.i.i.i.i.i.i.i.i = phi i64 [ %_5.sroa.6.i.i.i.i.i.i.i.i.i.i.i.i.sroa.8.3.i.i.i.i.i.i.i.i.i, %bb14.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i ], [ %_5.sroa.6.i.i.i.i.i.i.i.i.i.i.i.i.sroa.8.0133.i.i.i.i.i.i.i.i.i, %bb11.i.i.i.i.i.i.i.i.i ]
  %accum.sroa.4.011.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = phi ptr [ %_5.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, %bb14.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i ], [ %tmp.sroa.5.0.copyload.i.i.i.i.i.i.i.i.i, %bb11.i.i.i.i.i.i.i.i.i ]
  %17 = phi <2 x i32> [ %42, %bb14.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i ], [ %15, %bb11.i.i.i.i.i.i.i.i.i ]
  %18 = phi <2 x ptr> [ %43, %bb14.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i ], [ %16, %bb11.i.i.i.i.i.i.i.i.i ]
  %tmp.sroa.0.0.copyload.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = load i64, ptr %accum.sroa.4.011.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, align 8, !noalias !160861
  %_5.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = getelementptr i8, ptr %accum.sroa.4.011.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, i64 80
  call void @llvm.lifetime.start.p0(ptr nonnull %_5.sroa.6.i.i.i.i.i.i.i.i.i.i.i.i.sroa.0.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.start.p0(ptr nonnull %_5.sroa.9.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i)
  %.not.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = icmp eq i64 %tmp.sroa.0.0.copyload.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, -1
  br i1 %.not.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, label %bb14.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, label %bb3.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i

bb3.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i:  ; preds = %bb11.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i
  %tmp.sroa.5.0.self1.sroa_idx.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %accum.sroa.4.011.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, i64 8
  call void @llvm.lifetime.start.p0(ptr nonnull %_5.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i), !noalias !160864
  store i64 %tmp.sroa.0.0.copyload.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, ptr %_5.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, align 8, !noalias !160871
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %_8.sroa.5.0._5.i.sroa_idx.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, ptr noundef nonnull align 8 dereferenceable(72) %tmp.sroa.5.0.self1.sroa_idx.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, i64 72, i1 false), !noalias !160861
  call void @llvm.lifetime.start.p0(ptr nonnull %_2.i3.i.i.i.i.i.i.i.i.i), !noalias !160872
; invoke <purrdf_core::ir::term::TermValue>::into_rdf_term
  invoke void @<purrdf_core::ir::term::TermValue>::into_rdf_term(ptr noalias nofree noundef nonnull sret([80 x i8]) align 8 captures(address) dereferenceable(80) %_2.i3.i.i.i.i.i.i.i.i.i, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(80) %_5.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i)
          to label %.noexc.i.i.i.i.i.i.i.i.i unwind label %bb4.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, !noalias !160810

.noexc.i.i.i.i.i.i.i.i.i:                         ; preds = %bb3.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i
  %19 = load i64, ptr %_2.i3.i.i.i.i.i.i.i.i.i, align 8, !range !4141, !noalias !160872, !noundef !3995
  %20 = icmp eq i64 %19, -1
  br i1 %20, label %bb4.i4.i.i.i.i.i.i.i.i.i, label %bb1.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i

bb4.i4.i.i.i.i.i.i.i.i.i:                         ; preds = %.noexc.i.i.i.i.i.i.i.i.i
  call void @llvm.lifetime.start.p0(ptr nonnull %buf.i.i.i.i.i.i.i.i.i.i.i.i), !noalias !160876
  store i64 0, ptr %buf.i.i.i.i.i.i.i.i.i.i.i.i, align 8, !noalias !160876
  store ptr inttoptr (i64 1 to ptr), ptr %_10.sroa.4.0.buf.sroa_idx.i.i.i.i.i.i.i.i.i.i.i.i, align 8, !noalias !160876
  store i64 0, ptr %_10.sroa.5.0.buf.sroa_idx.i.i.i.i.i.i.i.i.i.i.i.i, align 8, !noalias !160876
  call void @llvm.lifetime.start.p0(ptr nonnull %formatter.i.i.i.i.i.i.i.i.i.i.i.i), !noalias !160876
  store i64 1610612768, ptr %6, align 8, !noalias !160876
  store ptr %buf.i.i.i.i.i.i.i.i.i.i.i.i, ptr %formatter.i.i.i.i.i.i.i.i.i.i.i.i, align 8, !noalias !160876
  store ptr @vtable.1T, ptr %7, align 8, !noalias !160876
; invoke <purrdf_core::ir::term::NonIriPredicate as core::fmt::Display>::fmt
  %_8.i.i.i.i.i.i.i.i.i.i.i.i = invoke noundef zeroext i1 @<purrdf_core::ir::term::NonIriPredicate as core::fmt::Display>::fmt(ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) %e.i.i.i.i.i.i.i.i.i.i.i.i.i, ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %formatter.i.i.i.i.i.i.i.i.i.i.i.i)
          to label %bb1.i.i.i5.i.i.i.i.i.i.i.i.i unwind label %cleanup.i3.i.i.loopexit.i.i.i.i.i.i.i.i.i, !noalias !160882

cleanup.i3.i.i.loopexit.i.i.i.i.i.i.i.i.i:        ; preds = %bb4.i4.i.i.i.i.i.i.i.i.i
  %lpad.loopexit.i.i.i.i.i.i.i.i.i = landingpad { ptr, i32 }
          cleanup
  store ptr %_49.i.i.i.i.i.i.i.i.i, ptr %_8.sroa.4.0._2.i.i.i.sroa_idx, align 8, !alias.scope !160883, !noalias !160884
  br label %cleanup.i3.i.i.i.i.i.i.i.i.i.i.i

cleanup.i3.i.i.loopexit.split-lp.i.i.i.i.i.i.i.i.i: ; preds = %bb2.i.i.i.i.i.i.i.i.i.i.i.i.i
  %lpad.loopexit.split-lp.i.i.i.i.i.i.i.i.i = landingpad { ptr, i32 }
          cleanup
  br label %cleanup.i3.i.i.i.i.i.i.i.i.i.i.i

cleanup.i3.i.i.i.i.i.i.i.i.i.i.i:                 ; preds = %cleanup.i3.i.i.loopexit.split-lp.i.i.i.i.i.i.i.i.i, %cleanup.i3.i.i.loopexit.i.i.i.i.i.i.i.i.i
  %lpad.phi.i.i.i.i.i.i.i.i.i = phi { ptr, i32 } [ %lpad.loopexit.i.i.i.i.i.i.i.i.i, %cleanup.i3.i.i.loopexit.i.i.i.i.i.i.i.i.i ], [ %lpad.loopexit.split-lp.i.i.i.i.i.i.i.i.i, %cleanup.i3.i.i.loopexit.split-lp.i.i.i.i.i.i.i.i.i ]
  call void @llvm.experimental.noalias.scope.decl(metadata !160885), !noalias !160888
  %_1.val.i.i.i.i.i.i.i.i.i.i.i.i.i = load i64, ptr %buf.i.i.i.i.i.i.i.i.i.i.i.i, align 8, !alias.scope !160885, !noalias !160876
  %21 = icmp eq i64 %_1.val.i.i.i.i.i.i.i.i.i.i.i.i.i, 0
  br i1 %21, label %bb4.i.i.i.i.i.i.i.i.i.i.i.i.body.i.i.i.i.i.i.i.i.i, label %bb2.i.i.i4.i.i.i.i.i.i.i.i.i.i.i.i.i.i

bb2.i.i.i4.i.i.i.i.i.i.i.i.i.i.i.i.i.i:           ; preds = %cleanup.i3.i.i.i.i.i.i.i.i.i.i.i
  %_1.val1.i.i.i.i.i.i.i.i.i.i.i.i.i = load ptr, ptr %_10.sroa.4.0.buf.sroa_idx.i.i.i.i.i.i.i.i.i.i.i.i, align 8, !alias.scope !160885, !noalias !160876, !nonnull !3995, !noundef !3995
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %_1.val1.i.i.i.i.i.i.i.i.i.i.i.i.i, i64 noundef %_1.val.i.i.i.i.i.i.i.i.i.i.i.i.i, i64 noundef range(i64 1, -9223372036854775807) 1) #77, !noalias !160889
  br label %bb4.i.i.i.i.i.i.i.i.i.i.i.i.body.i.i.i.i.i.i.i.i.i

bb1.i.i.i5.i.i.i.i.i.i.i.i.i:                     ; preds = %bb4.i4.i.i.i.i.i.i.i.i.i
  br i1 %_8.i.i.i.i.i.i.i.i.i.i.i.i, label %bb2.i.i.i.i.i.i.i.i.i.i.i.i.i, label %<purrdf_core::ir::term::NonIriPredicate as alloc::string::SpecToString>::spec_to_string (.exit.i.i.i.i.i.i.i.i.i.i.i), !prof !4025

bb2.i.i.i.i.i.i.i.i.i.i.i.i.i:                    ; preds = %bb1.i.i.i5.i.i.i.i.i.i.i.i.i
  store ptr %_49.i.i.i.i.i.i.i.i.i, ptr %_8.sroa.4.0._2.i.i.i.sroa_idx, align 8, !alias.scope !160883, !noalias !160884
; invoke core::result::unwrap_failed
  invoke void @core::result::unwrap_failed(ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @alloc_cc656815297f75969399c3f4b1ad3de4, i64 noundef 55, ptr noundef nonnull %e.i.i.i.i.i.i.i.i.i.i.i.i.i, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(32) @vtable.2q, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(24) @alloc_d4c8062c4f28c49e31e589e7f415a063) #78
          to label %.noexc.i.i.i.i.i.i.i.i.i.i.i.i unwind label %cleanup.i3.i.i.loopexit.split-lp.i.i.i.i.i.i.i.i.i, !noalias !160882

.noexc.i.i.i.i.i.i.i.i.i.i.i.i:                   ; preds = %bb2.i.i.i.i.i.i.i.i.i.i.i.i.i
  unreachable

<purrdf_core::ir::term::NonIriPredicate as alloc::string::SpecToString>::spec_to_string (.exit.i.i.i.i.i.i.i.i.i.i.i): ; preds = %bb1.i.i.i5.i.i.i.i.i.i.i.i.i
  %_3.sroa.0.0.copyload.i.i.i.i.i.i.i.i.i.i.i = load i64, ptr %buf.i.i.i.i.i.i.i.i.i.i.i.i, align 8, !noalias !160890
  %_3.sroa.4.0.copyload.i.i.i.i.i.i.i.i.i.i.i = load ptr, ptr %_10.sroa.4.0.buf.sroa_idx.i.i.i.i.i.i.i.i.i.i.i.i, align 8, !noalias !160890
  %_3.sroa.5.0.copyload.i.i.i.i.i.i.i.i.i.i.i = load i64, ptr %_10.sroa.5.0.buf.sroa_idx.i.i.i.i.i.i.i.i.i.i.i.i, align 8, !noalias !160890
  call void @llvm.lifetime.end.p0(ptr nonnull %formatter.i.i.i.i.i.i.i.i.i.i.i.i), !noalias !160876
  call void @llvm.lifetime.end.p0(ptr nonnull %buf.i.i.i.i.i.i.i.i.i.i.i.i), !noalias !160876
; call __rustc::__rust_no_alloc_shim_is_unstable_v2
  call void @__rustc::__rust_no_alloc_shim_is_unstable_v2() #77, !noalias !160891
; call __rustc::__rust_alloc
  %22 = call noundef align 8 dereferenceable_or_null(24) ptr @__rustc::__rust_alloc(i64 noundef 24, i64 noundef range(i64 1, -9223372036854775807) 8) #77, !noalias !160891
  %23 = icmp eq ptr %22, null
  br i1 %23, label %bb2.i5.i.i.i.i.i.i.i.i.i.i.i, label %bb3.i2.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, !prof !4936

bb2.i5.i.i.i.i.i.i.i.i.i.i.i:                     ; preds = %<purrdf_core::ir::term::NonIriPredicate as alloc::string::SpecToString>::spec_to_string (.exit.i.i.i.i.i.i.i.i.i.i.i)
  store ptr %_49.i.i.i.i.i.i.i.i.i, ptr %_8.sroa.4.0._2.i.i.i.sroa_idx, align 8, !alias.scope !160883, !noalias !160884
; invoke alloc::alloc::handle_alloc_error
  invoke void @alloc::alloc::handle_alloc_error(i64 noundef 8, i64 noundef 24) #80
          to label %.noexc.i.i.i.i.i.i.i.i.i.i.i unwind label %cleanup.i.i.i6.i.i.i.i.i.i.i.i.i, !noalias !160894

.noexc.i.i.i.i.i.i.i.i.i.i.i:                     ; preds = %bb2.i5.i.i.i.i.i.i.i.i.i.i.i
  unreachable

cleanup.i.i.i6.i.i.i.i.i.i.i.i.i:                 ; preds = %bb2.i5.i.i.i.i.i.i.i.i.i.i.i
  %24 = landingpad { ptr, i32 }
          cleanup
  %25 = icmp eq i64 %_3.sroa.0.0.copyload.i.i.i.i.i.i.i.i.i.i.i, 0
  br i1 %25, label %bb4.i.i.i.i.i.i.i.i.i.i.i.i.body.i.i.i.i.i.i.i.i.i, label %bb2.i.i.i4.i.i.i7.i.i.i.i.i.i.i.i.i.i.i

bb2.i.i.i4.i.i.i7.i.i.i.i.i.i.i.i.i.i.i:          ; preds = %cleanup.i.i.i6.i.i.i.i.i.i.i.i.i
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %_3.sroa.4.0.copyload.i.i.i.i.i.i.i.i.i.i.i) ], !noalias !160888
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %_3.sroa.4.0.copyload.i.i.i.i.i.i.i.i.i.i.i, i64 noundef %_3.sroa.0.0.copyload.i.i.i.i.i.i.i.i.i.i.i, i64 noundef range(i64 1, -9223372036854775807) 1) #77, !noalias !160895
  br label %bb4.i.i.i.i.i.i.i.i.i.i.i.i.body.i.i.i.i.i.i.i.i.i

bb1.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i:    ; preds = %.noexc.i.i.i.i.i.i.i.i.i
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %_5.sroa.6.i.i.i.i.i.i.i.i.i.i.i.i.sroa.0.i.i.i.i.i.i.i.i.i, ptr noundef nonnull align 8 dereferenceable(16) %_6.i.i.i.i.i.i.i.i.i.i.i.i.i.sroa.7.0._2.i3.sroa_idx.i.i.i.i.i.i.i.i.i, i64 16, i1 false), !noalias !160900
  %_6.i.i.i.i.i.i.i.i.i.i.i.i.i.sroa.9.0.copyload.i.i.i.i.i.i.i.i.i = load i64, ptr %_6.i.i.i.i.i.i.i.i.i.i.i.i.i.sroa.9.0._2.i3.sroa_idx.i.i.i.i.i.i.i.i.i, align 8, !noalias !160901
  %26 = load <2 x ptr>, ptr %_6.i.i.i.i.i.i.i.i.i.i.i.i.i.sroa.10.0._2.i3.sroa_idx.i.i.i.i.i.i.i.i.i, align 8, !noalias !160901
  %_6.i.i.i.i.i.i.i.i.i.i.i.i.i.sroa.13.sroa.0.0.copyload.i.i.i.i.i.i.i.i.i = load i32, ptr %_6.i.i.i.i.i.i.i.i.i.i.i.i.i.sroa.13.0._2.i3.sroa_idx.i.i.i.i.i.i.i.i.i, align 4, !noalias !160901
  %27 = load <2 x i32>, ptr %_6.i.i.i.i.i.i.i.i.i.i.i.i.i.sroa.12.0._2.i3.sroa_idx.i.i.i.i.i.i.i.i.i, align 8, !noalias !160901
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %_5.sroa.9.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, ptr noundef nonnull align 8 dereferenceable(24) %_6.i.i.i.i.i.i.i.i.i.i.i.i.i.sroa.13.sroa.6.0._6.i.i.i.i.i.i.i.i.i.i.i.i.i.sroa.13.0._2.i3.sroa_idx.sroa_idx.i.i.i.i.i.i.i.i.i, i64 24, i1 false), !noalias !160900
  call void @llvm.lifetime.end.p0(ptr nonnull %_2.i3.i.i.i.i.i.i.i.i.i), !noalias !160872
  call void @llvm.lifetime.end.p0(ptr nonnull %_5.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i), !noalias !160864
  br label %bb14.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i

bb3.i2.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i: ; preds = %<purrdf_core::ir::term::NonIriPredicate as alloc::string::SpecToString>::spec_to_string (.exit.i.i.i.i.i.i.i.i.i.i.i)
  store i64 %_3.sroa.0.0.copyload.i.i.i.i.i.i.i.i.i.i.i, ptr %22, align 8, !noalias !160894
  %_8.sroa.5.0..sroa_idx.i.i.i.i.i.i.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %22, i64 8
  store ptr %_3.sroa.4.0.copyload.i.i.i.i.i.i.i.i.i.i.i, ptr %_8.sroa.5.0..sroa_idx.i.i.i.i.i.i.i.i.i.i.i, align 8, !noalias !160894
  %_8.sroa.6.0..sroa_idx.i.i.i.i.i.i.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %22, i64 16
  store i64 %_3.sroa.5.0.copyload.i.i.i.i.i.i.i.i.i.i.i, ptr %_8.sroa.6.0..sroa_idx.i.i.i.i.i.i.i.i.i.i.i, align 8, !noalias !160894
  call void @llvm.lifetime.end.p0(ptr nonnull %_2.i3.i.i.i.i.i.i.i.i.i), !noalias !160872
  call void @llvm.lifetime.end.p0(ptr nonnull %_5.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i), !noalias !160864
  store ptr %_5.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, ptr %_3.sroa.4.0._2.i.i.i.sroa_idx.i.i.i.i.i.i.i.i.i.i.i, align 8, !alias.scope !160902, !noalias !160903
  %_2.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = load i64, ptr %residual.i.i.i.i.i.i.i.i.i.i.i.i, align 8, !range !5056, !alias.scope !160904, !noalias !160907, !noundef !3995
  %28 = icmp eq i64 %_2.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, 0
  br i1 %28, label %bb7.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, label %bb2.i6.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i

bb2.i6.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i: ; preds = %bb3.i2.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i
  call void @llvm.experimental.noalias.scope.decl(metadata !160911)
  call void @llvm.experimental.noalias.scope.decl(metadata !160914), !noalias !160917
  call void @llvm.experimental.noalias.scope.decl(metadata !160918), !noalias !160917
  call void @llvm.experimental.noalias.scope.decl(metadata !160921), !noalias !160917
  %_2.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = load i64, ptr %8, align 8, !range !5056, !alias.scope !160924, !noalias !160907, !noundef !3995
  %29 = icmp eq i64 %_2.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, 0
  br i1 %29, label %bb7.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, label %bb2.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i

bb2.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i:                ; preds = %bb2.i6.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i
  %.val.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = load ptr, ptr %9, align 8, !alias.scope !160924, !noalias !160907, !noundef !3995
  %.val1.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = load ptr, ptr %10, align 8, !alias.scope !160924, !noalias !160907, !nonnull !3995, !noundef !3995
  %.not.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = icmp eq ptr %.val.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, null
  br i1 %.not.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, label %bb3.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, label %bb2.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i

bb2.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i:              ; preds = %bb2.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i
  %30 = load ptr, ptr %.val1.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, align 8, !invariant.load !3995, !noalias !160925
  %.not.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = icmp eq ptr %30, null
  br i1 %.not.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, label %bb3.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, label %is_not_null.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i

is_not_null.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i:    ; preds = %bb2.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i
  invoke void %30(ptr noundef nonnull %.val.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i)
          to label %bb3.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i unwind label %cleanup.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, !noalias !160925

bb3.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i:            ; preds = %is_not_null.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, %bb2.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i
  %31 = getelementptr inbounds nuw i8, ptr %.val1.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, i64 8
  %size.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = load i64, ptr %31, align 8, !range !4210, !invariant.load !3995, !noalias !160925
  %32 = icmp eq i64 %size.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, 0
  br i1 %32, label %bb7.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, label %<alloc::alloc::Global as core::alloc::Allocator>::deallocate (.exit.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i)

<alloc::alloc::Global as core::alloc::Allocator>::deallocate (.exit.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i): ; preds = %bb3.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i
  %33 = getelementptr inbounds nuw i8, ptr %.val1.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, i64 16
  %align.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = load i64, ptr %33, align 8, !range !3997, !invariant.load !3995, !noalias !160925
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %.val.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, i64 noundef %size.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, i64 noundef range(i64 1, -9223372036854775807) %align.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i) #77, !noalias !160925
  br label %bb7.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i

cleanup.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i:        ; preds = %is_not_null.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i
  %34 = landingpad { ptr, i32 }
          cleanup
  store ptr %_49.i.i.i.i.i.i.i.i.i, ptr %_8.sroa.4.0._2.i.i.i.sroa_idx, align 8, !alias.scope !160883, !noalias !160884
  %35 = getelementptr inbounds nuw i8, ptr %.val1.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, i64 8
  %size.i4.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = load i64, ptr %35, align 8, !range !4210, !invariant.load !3995, !noalias !160925
  %36 = icmp eq i64 %size.i4.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, 0
  br i1 %36, label %bb11.i.i.i.i.i.i.i.i.i.i.i.body.i.i.i.i.i.i.i.i.i.i.i, label %<alloc::alloc::Global as core::alloc::Allocator>::deallocate (.exit.i5.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i)

<alloc::alloc::Global as core::alloc::Allocator>::deallocate (.exit.i5.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i): ; preds = %cleanup.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i
  %37 = getelementptr inbounds nuw i8, ptr %.val1.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, i64 16
  %align.i6.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = load i64, ptr %37, align 8, !range !3997, !invariant.load !3995, !noalias !160925
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %.val.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, i64 noundef %size.i4.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, i64 noundef range(i64 1, -9223372036854775807) %align.i6.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i) #77, !noalias !160925
  br label %bb11.i.i.i.i.i.i.i.i.i.i.i.body.i.i.i.i.i.i.i.i.i.i.i

bb3.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i:              ; preds = %bb2.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i
  %self3.val.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = load i64, ptr %_3.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, align 8, !noalias !160925, !noundef !3995
  %_0.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = icmp sgt i64 %self3.val.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, 0
  br i1 %_0.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, label %bb1.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, label %bb2.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, !prof !7466

bb2.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i:      ; preds = %bb3.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i
; invoke <pyo3::instance::Py<_> as core::ops::drop::Drop>::drop::drop_slow
  invoke void @<pyo3::instance::Py<_> as core::ops::drop::Drop>::drop::drop_slow(ptr noundef nonnull %.val1.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i)
          to label %bb7.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i unwind label %bb11.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, !noalias !160926

bb1.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i:      ; preds = %bb3.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i
  call void @_Py_DecRef(ptr noundef nonnull %.val1.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i) #77, !noalias !160925
  br label %bb7.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i

bb11.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i: ; preds = %bb2.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i
  %38 = landingpad { ptr, i32 }
          cleanup
  store ptr %_49.i.i.i.i.i.i.i.i.i, ptr %_8.sroa.4.0._2.i.i.i.sroa_idx, align 8, !alias.scope !160883, !noalias !160884
  br label %bb11.i.i.i.i.i.i.i.i.i.i.i.body.i.i.i.i.i.i.i.i.i.i.i

bb11.i.i.i.i.i.i.i.i.i.i.i.body.i.i.i.i.i.i.i.i.i.i.i: ; preds = %bb11.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, %<alloc::alloc::Global as core::alloc::Allocator>::deallocate (.exit.i5.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i), %cleanup.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i
  %eh.lpad-body.i.i.i.i.i.i.i.i.i.i.i = phi { ptr, i32 } [ %38, %bb11.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i ], [ %34, %<alloc::alloc::Global as core::alloc::Allocator>::deallocate (.exit.i5.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i) ], [ %34, %cleanup.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i ]
  store i64 1, ptr %residual.i.i.i.i.i.i.i.i.i.i.i.i, align 8, !noalias !160907
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %_11.sroa.5.0._16.sroa_idx.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, i8 0, i64 16, i1 false), !noalias !160810
  store i64 1, ptr %8, align 8, !noalias !160927
  store ptr %22, ptr %9, align 8, !noalias !160927
  store ptr @vtable.22, ptr %10, align 8, !noalias !160927
  store i32 3, ptr %_5.sroa.6.i.i.i.i.i.i.i.i.i.i.i.i.sroa.14.0._11.sroa.5.0._16.sroa_idx.i.i.i.i.i.i.i.i.i.i.i.i.i.sroa_idx.i.i.i.i.i.i.i.i.i, align 8, !noalias !160927
  store i32 %_6.i.i.i.i.i.i.i.i.i.i.i.i.i.sroa.13.sroa.0.1.i.i.i.i.i.i.i.i.i, ptr %_5.sroa.6.i.i.i.i.i.i.i.i.i.i.i.i.sroa.16.0._11.sroa.5.0._16.sroa_idx.i.i.i.i.i.i.i.i.i.i.i.i.i.sroa_idx.i.i.i.i.i.i.i.i.i, align 4, !noalias !160927
; invoke core::ptr::drop_glue::<alloc::vec::in_place_drop::InPlaceDrop<core::option::Option<purrdf_core::model::RdfTerm>>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::in_place_drop::InPlaceDrop<core::option::Option<purrdf_core::model::RdfTerm>>>(ptr nonnull %tmp.sroa.5.0.copyload.i.i.i.i.i.i.i.i.i, ptr nonnull %accum.sroa.4.011.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i) #79
          to label %bb14.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i unwind label %terminate.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, !noalias !160907

terminate.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i: ; preds = %bb11.i.i.i.i.i.i.i.i.i.i.i.body.i.i.i.i.i.i.i.i.i.i.i
  %39 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #75, !noalias !160907
  unreachable

bb4.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i:    ; preds = %bb3.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i
  %40 = landingpad { ptr, i32 }
          cleanup
  store ptr %_49.i.i.i.i.i.i.i.i.i, ptr %_8.sroa.4.0._2.i.i.i.sroa_idx, align 8, !alias.scope !160883, !noalias !160884
  br label %bb4.i.i.i.i.i.i.i.i.i.i.i.i.body.i.i.i.i.i.i.i.i.i

bb4.i.i.i.i.i.i.i.i.i.i.i.i.body.i.i.i.i.i.i.i.i.i: ; preds = %bb4.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, %bb2.i.i.i4.i.i.i7.i.i.i.i.i.i.i.i.i.i.i, %cleanup.i.i.i6.i.i.i.i.i.i.i.i.i, %bb2.i.i.i4.i.i.i.i.i.i.i.i.i.i.i.i.i.i, %cleanup.i3.i.i.i.i.i.i.i.i.i.i.i
  %eh.lpad-body.i.i.i.i.i.i.i.i.i = phi { ptr, i32 } [ %40, %bb4.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i ], [ %lpad.phi.i.i.i.i.i.i.i.i.i, %cleanup.i3.i.i.i.i.i.i.i.i.i.i.i ], [ %lpad.phi.i.i.i.i.i.i.i.i.i, %bb2.i.i.i4.i.i.i.i.i.i.i.i.i.i.i.i.i.i ], [ %24, %bb2.i.i.i4.i.i.i7.i.i.i.i.i.i.i.i.i.i.i ], [ %24, %cleanup.i.i.i6.i.i.i.i.i.i.i.i.i ]
  store ptr %_5.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, ptr %_3.sroa.4.0._2.i.i.i.sroa_idx.i.i.i.i.i.i.i.i.i.i.i, align 8, !alias.scope !160902, !noalias !160903
; invoke core::ptr::drop_glue::<alloc::vec::in_place_drop::InPlaceDrop<core::option::Option<purrdf_core::model::RdfTerm>>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::in_place_drop::InPlaceDrop<core::option::Option<purrdf_core::model::RdfTerm>>>(ptr nonnull %tmp.sroa.5.0.copyload.i.i.i.i.i.i.i.i.i, ptr nonnull %accum.sroa.4.011.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i) #79
          to label %bb14.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i unwind label %terminate.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, !noalias !160900

terminate.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i: ; preds = %bb4.i.i.i.i.i.i.i.i.i.i.i.i.body.i.i.i.i.i.i.i.i.i
  %41 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #75, !noalias !160900
  unreachable

bb14.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i:     ; preds = %bb1.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, %bb11.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i
  %_6.i.i.i.i.i.i.i.i.i.i.i.i.i.sroa.13.sroa.0.3.i.i.i.i.i.i.i.i.i = phi i32 [ %_6.i.i.i.i.i.i.i.i.i.i.i.i.i.sroa.13.sroa.0.0.copyload.i.i.i.i.i.i.i.i.i, %bb1.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i ], [ %_6.i.i.i.i.i.i.i.i.i.i.i.i.i.sroa.13.sroa.0.1.i.i.i.i.i.i.i.i.i, %bb11.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i ]
  %_5.sroa.6.i.i.i.i.i.i.i.i.i.i.i.i.sroa.8.3.i.i.i.i.i.i.i.i.i = phi i64 [ %_6.i.i.i.i.i.i.i.i.i.i.i.i.i.sroa.9.0.copyload.i.i.i.i.i.i.i.i.i, %bb1.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i ], [ %_5.sroa.6.i.i.i.i.i.i.i.i.i.i.i.i.sroa.8.1.i.i.i.i.i.i.i.i.i, %bb11.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i ]
  %_7.sroa.0.09.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = phi i64 [ %19, %bb1.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i ], [ -1, %bb11.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i ]
  %42 = phi <2 x i32> [ %27, %bb1.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i ], [ %17, %bb11.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i ]
  %43 = phi <2 x ptr> [ %26, %bb1.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i ], [ %18, %bb11.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i ]
  store i64 %_7.sroa.0.09.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, ptr %accum.sroa.4.011.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, align 8, !noalias !160907
  %_8.sroa.6.16.acc.sroa.4.8._9.1.sroa_idx.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %accum.sroa.4.011.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %_8.sroa.6.16.acc.sroa.4.8._9.1.sroa_idx.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, ptr noundef nonnull align 8 dereferenceable(16) %_5.sroa.6.i.i.i.i.i.i.i.i.i.i.i.i.sroa.0.i.i.i.i.i.i.i.i.i, i64 16, i1 false), !noalias !160927
  %_5.sroa.6.i.i.i.i.i.i.i.i.i.i.i.i.sroa.8.0._8.sroa.6.16.acc.sroa.4.8._9.1.sroa_idx.i.i.i.i.i.i.i.i.i.i.i.i.i.sroa_idx.i.i.i.i.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %accum.sroa.4.011.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, i64 24
  store i64 %_5.sroa.6.i.i.i.i.i.i.i.i.i.i.i.i.sroa.8.3.i.i.i.i.i.i.i.i.i, ptr %_5.sroa.6.i.i.i.i.i.i.i.i.i.i.i.i.sroa.8.0._8.sroa.6.16.acc.sroa.4.8._9.1.sroa_idx.i.i.i.i.i.i.i.i.i.i.i.i.i.sroa_idx.i.i.i.i.i.i.i.i.i, align 8, !noalias !160927
  %_5.sroa.6.i.i.i.i.i.i.i.i.i.i.i.i.sroa.10.0._8.sroa.6.16.acc.sroa.4.8._9.1.sroa_idx.i.i.i.i.i.i.i.i.i.i.i.i.i.sroa_idx.i.i.i.i.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %accum.sroa.4.011.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, i64 32
  store <2 x ptr> %43, ptr %_5.sroa.6.i.i.i.i.i.i.i.i.i.i.i.i.sroa.10.0._8.sroa.6.16.acc.sroa.4.8._9.1.sroa_idx.i.i.i.i.i.i.i.i.i.i.i.i.i.sroa_idx.i.i.i.i.i.i.i.i.i, align 8, !noalias !160927
  %_5.sroa.6.i.i.i.i.i.i.i.i.i.i.i.i.sroa.14.0._8.sroa.6.16.acc.sroa.4.8._9.1.sroa_idx.i.i.i.i.i.i.i.i.i.i.i.i.i.sroa_idx.i.i.i.i.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %accum.sroa.4.011.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, i64 48
  store <2 x i32> %42, ptr %_5.sroa.6.i.i.i.i.i.i.i.i.i.i.i.i.sroa.14.0._8.sroa.6.16.acc.sroa.4.8._9.1.sroa_idx.i.i.i.i.i.i.i.i.i.i.i.i.i.sroa_idx.i.i.i.i.i.i.i.i.i, align 8, !noalias !160927
  %_8.sroa.7.16.acc.sroa.4.8._9.1.sroa_idx.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %accum.sroa.4.011.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, i64 56
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %_8.sroa.7.16.acc.sroa.4.8._9.1.sroa_idx.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, ptr noundef nonnull align 8 dereferenceable(24) %_5.sroa.9.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, i64 24, i1 false), !noalias !160927
  call void @llvm.lifetime.end.p0(ptr nonnull %_5.sroa.6.i.i.i.i.i.i.i.i.i.i.i.i.sroa.0.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.end.p0(ptr nonnull %_5.sroa.9.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i)
  %_44.not.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = icmp eq ptr %_5.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, %_8.i.i.i.i.i.i.i.i.i.i.i
  br i1 %_44.not.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, label %bb6.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, label %bb11.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i

bb7.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i:          ; preds = %bb1.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, %bb2.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, %<alloc::alloc::Global as core::alloc::Allocator>::deallocate (.exit.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i), %bb3.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, %bb2.i6.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, %bb3.i2.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i
  store i64 1, ptr %residual.i.i.i.i.i.i.i.i.i.i.i.i, align 8, !noalias !160907
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %_11.sroa.5.0._16.sroa_idx.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, i8 0, i64 16, i1 false), !noalias !160810
  store i64 1, ptr %8, align 8, !noalias !160927
  store ptr %22, ptr %9, align 8, !noalias !160927
  store ptr @vtable.22, ptr %10, align 8, !noalias !160927
  store i32 3, ptr %_5.sroa.6.i.i.i.i.i.i.i.i.i.i.i.i.sroa.14.0._11.sroa.5.0._16.sroa_idx.i.i.i.i.i.i.i.i.i.i.i.i.i.sroa_idx.i.i.i.i.i.i.i.i.i, align 8, !noalias !160927
  store i32 %_6.i.i.i.i.i.i.i.i.i.i.i.i.i.sroa.13.sroa.0.1.i.i.i.i.i.i.i.i.i, ptr %_5.sroa.6.i.i.i.i.i.i.i.i.i.i.i.i.sroa.16.0._11.sroa.5.0._16.sroa_idx.i.i.i.i.i.i.i.i.i.i.i.i.i.sroa_idx.i.i.i.i.i.i.i.i.i, align 4, !noalias !160927
  call void @llvm.lifetime.end.p0(ptr nonnull %_5.sroa.6.i.i.i.i.i.i.i.i.i.i.i.i.sroa.0.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.end.p0(ptr nonnull %_5.sroa.9.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i)
  %44 = insertelement <2 x i32> <i32 3, i32 poison>, i32 %_6.i.i.i.i.i.i.i.i.i.i.i.i.i.sroa.13.sroa.0.1.i.i.i.i.i.i.i.i.i, i64 1
  %45 = insertelement <2 x ptr> <ptr poison, ptr @vtable.22>, ptr %22, i64 0
  br label %bb6.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i

bb6.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i:              ; preds = %bb14.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, %bb7.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, %bb11.i.i.i.i.i.i.i.i.i
  %_6.i.i.i.i.i.i.i.i.i.i.i.i.i.sroa.13.sroa.0.4.i.i.i.i.i.i.i.i.i = phi i32 [ %_6.i.i.i.i.i.i.i.i.i.i.i.i.i.sroa.13.sroa.0.0132.i.i.i.i.i.i.i.i.i, %bb11.i.i.i.i.i.i.i.i.i ], [ %_6.i.i.i.i.i.i.i.i.i.i.i.i.i.sroa.13.sroa.0.1.i.i.i.i.i.i.i.i.i, %bb7.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i ], [ %_6.i.i.i.i.i.i.i.i.i.i.i.i.i.sroa.13.sroa.0.3.i.i.i.i.i.i.i.i.i, %bb14.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i ]
  %_5.sroa.6.i.i.i.i.i.i.i.i.i.i.i.i.sroa.8.4.i.i.i.i.i.i.i.i.i = phi i64 [ %_5.sroa.6.i.i.i.i.i.i.i.i.i.i.i.i.sroa.8.0133.i.i.i.i.i.i.i.i.i, %bb11.i.i.i.i.i.i.i.i.i ], [ 1, %bb7.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i ], [ %_5.sroa.6.i.i.i.i.i.i.i.i.i.i.i.i.sroa.8.3.i.i.i.i.i.i.i.i.i, %bb14.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i ]
  %self.val.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = phi ptr [ %tmp.sroa.5.0.copyload.i.i.i.i.i.i.i.i.i, %bb11.i.i.i.i.i.i.i.i.i ], [ %_5.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, %bb7.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i ], [ %_8.i.i.i.i.i.i.i.i.i.i.i, %bb14.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i ]
  %accum.sroa.4.011.i.i.pn.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = phi ptr [ %tmp.sroa.5.0.copyload.i.i.i.i.i.i.i.i.i, %bb11.i.i.i.i.i.i.i.i.i ], [ %accum.sroa.4.011.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, %bb7.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i ], [ %_8.i.i.i.i.i.i.i.i.i.i.i, %bb14.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i ]
  %46 = phi <2 x i32> [ %15, %bb11.i.i.i.i.i.i.i.i.i ], [ %44, %bb7.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i ], [ %42, %bb14.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i ]
  %47 = phi <2 x ptr> [ %16, %bb11.i.i.i.i.i.i.i.i.i ], [ %45, %bb7.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i ], [ %43, %bb14.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i ]
  %48 = ptrtoint ptr %accum.sroa.4.011.i.i.pn.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i to i64
  %49 = ptrtoint ptr %tmp.sroa.5.0.copyload.i.i.i.i.i.i.i.i.i to i64
  %50 = sub nuw i64 %48, %49
  %_0.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = udiv exact i64 %50, 80
  call void @llvm.lifetime.start.p0(ptr nonnull %dst_guard.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i), !noalias !160928
  store ptr %tmp.sroa.5.0.copyload.i.i.i.i.i.i.i.i.i, ptr %dst_guard.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, align 8, !noalias !160928
  store i64 %_0.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, ptr %11, align 8, !noalias !160928
  store i64 %tmp.sroa.0.0.copyload.i.i.i.i.i.i.i.i.i, ptr %12, align 8, !noalias !160928
  call void @llvm.experimental.noalias.scope.decl(metadata !160929)
  %51 = ptrtoint ptr %_8.i.i.i.i.i.i.i.i.i.i.i to i64
  %52 = ptrtoint ptr %self.val.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i to i64
  %53 = sub nuw i64 %51, %52
  %54 = udiv exact i64 %53, 80
  store i64 0, ptr %_3.sroa.5.0._2.i.i.i.sroa_idx.i.i.i.i.i.i.i.i.i.i.i, align 16, !alias.scope !160932, !noalias !160860
  store <2 x ptr> <ptr inttoptr (i64 8 to ptr), ptr inttoptr (i64 8 to ptr)>, ptr %_2.i.i.i.i.i.i.i.i.i.i.i.i.i.i, align 16, !alias.scope !160932, !noalias !160860
  store ptr inttoptr (i64 8 to ptr), ptr %_3.sroa.6.0._2.i.i.i.sroa_idx.i.i.i.i.i.i.i.i.i.i.i, align 8, !alias.scope !160932, !noalias !160860
  call void @llvm.experimental.noalias.scope.decl(metadata !160933)
  %_79.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = icmp eq ptr %_8.i.i.i.i.i.i.i.i.i.i.i, %self.val.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i
  br i1 %_79.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, label %<alloc::vec::Vec<core::option::Option<purrdf_core::model::RdfTerm>> as core::iter::traits::collect::FromIterator<core::option::Option<purrdf_core::model::RdfTerm>>>::from_iter::<core::iter::adapters::GenericShunt<core::iter::adapters::map::Map<alloc::vec::into_iter::IntoIter<core::option::Option<purrdf_core::ir::term::TermValue>>, purrdf_native::py_store::query::materialize_results::{closure#0}::{closure#0}>, core::result::Result<!, pyo3::err::PyErr>>> (.exit.i.i.i.i.i.i.i.i.i.i.i.i.i), label %bb5.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i

bb5.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i:          ; preds = %bb6.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, %core::ptr::drop_glue::<core::option::Option<purrdf_core::ir::term::TermValue>> (.exit.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i)
  %_3.sroa.0.010.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = phi i64 [ %55, %core::ptr::drop_glue::<core::option::Option<purrdf_core::ir::term::TermValue>> (.exit.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i) ], [ 0, %bb6.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i ]
  %_6.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = getelementptr inbounds nuw [80 x i8], ptr %self.val.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, i64 %_3.sroa.0.010.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i
  %55 = add nuw nsw i64 %_3.sroa.0.010.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, 1
  %56 = load i64, ptr %_6.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, align 8, !range !4141, !alias.scope !160936, !noalias !160939, !noundef !3995
  %57 = icmp eq i64 %56, -1
  br i1 %57, label %core::ptr::drop_glue::<core::option::Option<purrdf_core::ir::term::TermValue>> (.exit.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i), label %bb2.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i

bb2.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i:        ; preds = %bb5.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i
; invoke core::ptr::drop_glue::<purrdf_core::ir::term::TermValue>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_core::ir::term::TermValue>(ptr noalias nofree noundef nonnull align 8 dereferenceable(80) %_6.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i)
          to label %core::ptr::drop_glue::<core::option::Option<purrdf_core::ir::term::TermValue>> (.exit.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i) unwind label %cleanup.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, !noalias !160939

core::ptr::drop_glue::<core::option::Option<purrdf_core::ir::term::TermValue>> (.exit.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i): ; preds = %bb2.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, %bb5.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i
  %_7.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = icmp eq i64 %55, %54
  br i1 %_7.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, label %<alloc::vec::Vec<core::option::Option<purrdf_core::model::RdfTerm>> as core::iter::traits::collect::FromIterator<core::option::Option<purrdf_core::model::RdfTerm>>>::from_iter::<core::iter::adapters::GenericShunt<core::iter::adapters::map::Map<alloc::vec::into_iter::IntoIter<core::option::Option<purrdf_core::ir::term::TermValue>>, purrdf_native::py_store::query::materialize_results::{closure#0}::{closure#0}>, core::result::Result<!, pyo3::err::PyErr>>> (.exit.i.i.i.i.i.i.i.i.i.i.i.i.i), label %bb5.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i

cleanup.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i:      ; preds = %bb2.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i
  %58 = landingpad { ptr, i32 }
          cleanup
  store ptr %_49.i.i.i.i.i.i.i.i.i, ptr %_8.sroa.4.0._2.i.i.i.sroa_idx, align 8, !alias.scope !160883, !noalias !160884
  %_511.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = icmp eq i64 %55, %54
  br i1 %_511.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, label %cleanup3.body.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, label %bb3.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i

bb3.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i:          ; preds = %cleanup.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, %core::ptr::drop_glue::<core::option::Option<purrdf_core::ir::term::TermValue>> (.exit8.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i)
  %_3.sroa.0.112.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = phi i64 [ %59, %core::ptr::drop_glue::<core::option::Option<purrdf_core::ir::term::TermValue>> (.exit8.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i) ], [ %55, %cleanup.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i ]
  %_4.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = getelementptr inbounds nuw [80 x i8], ptr %self.val.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, i64 %_3.sroa.0.112.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i
  %59 = add i64 %_3.sroa.0.112.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, 1
  %60 = load i64, ptr %_4.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, align 8, !range !4141, !alias.scope !160940, !noalias !160939, !noundef !3995
  %61 = icmp eq i64 %60, -1
  br i1 %61, label %core::ptr::drop_glue::<core::option::Option<purrdf_core::ir::term::TermValue>> (.exit8.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i), label %bb2.i6.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i

bb2.i6.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i:       ; preds = %bb3.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i
; invoke core::ptr::drop_glue::<purrdf_core::ir::term::TermValue>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_core::ir::term::TermValue>(ptr noalias nofree noundef nonnull align 8 dereferenceable(80) %_4.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i)
          to label %core::ptr::drop_glue::<core::option::Option<purrdf_core::ir::term::TermValue>> (.exit8.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i) unwind label %terminate.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, !noalias !160939

core::ptr::drop_glue::<core::option::Option<purrdf_core::ir::term::TermValue>> (.exit8.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i): ; preds = %bb2.i6.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, %bb3.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i
  %_5.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = icmp eq i64 %59, %54
  br i1 %_5.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, label %cleanup3.body.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, label %bb3.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i

terminate.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i:    ; preds = %bb2.i6.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i
  %62 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #75, !noalias !160943
  unreachable

cleanup3.body.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i:    ; preds = %core::ptr::drop_glue::<core::option::Option<purrdf_core::ir::term::TermValue>> (.exit8.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i), %cleanup.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i
; invoke core::ptr::drop_glue::<alloc::vec::in_place_drop::InPlaceDstDataSrcBufDrop<core::option::Option<purrdf_core::ir::term::TermValue>, core::option::Option<purrdf_core::model::RdfTerm>>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::in_place_drop::InPlaceDstDataSrcBufDrop<core::option::Option<purrdf_core::ir::term::TermValue>, core::option::Option<purrdf_core::model::RdfTerm>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %dst_guard.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i) #79
          to label %bb14.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i unwind label %terminate.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, !noalias !160928

terminate.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i:        ; preds = %cleanup3.body.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, %bb14.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i
  %63 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #75, !noalias !160928
  unreachable

<alloc::vec::Vec<core::option::Option<purrdf_core::model::RdfTerm>> as core::iter::traits::collect::FromIterator<core::option::Option<purrdf_core::model::RdfTerm>>>::from_iter::<core::iter::adapters::GenericShunt<core::iter::adapters::map::Map<alloc::vec::into_iter::IntoIter<core::option::Option<purrdf_core::ir::term::TermValue>>, purrdf_native::py_store::query::materialize_results::{closure#0}::{closure#0}>, core::result::Result<!, pyo3::err::PyErr>>> (.exit.i.i.i.i.i.i.i.i.i.i.i.i.i): ; preds = %core::ptr::drop_glue::<core::option::Option<purrdf_core::ir::term::TermValue>> (.exit.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i), %bb6.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i
  store i64 %tmp.sroa.0.0.copyload.i.i.i.i.i.i.i.i.i, ptr %value.i.i.i.i.i.i.i.i.i.i.i.i, align 8, !alias.scope !160944, !noalias !160945
  store ptr %tmp.sroa.5.0.copyload.i.i.i.i.i.i.i.i.i, ptr %13, align 8, !alias.scope !160944, !noalias !160945
  store i64 %_0.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, ptr %14, align 8, !alias.scope !160944, !noalias !160945
  call void @llvm.lifetime.end.p0(ptr nonnull %dst_guard.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i), !noalias !160928
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<core::option::Option<purrdf_core::ir::term::TermValue>>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<core::option::Option<purrdf_core::ir::term::TermValue>>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(40) %_2.i.i.i.i.i.i.i.i.i.i.i.i.i.i)
          to label %bb1.i.i.i.i.i.i.i.i.i.i.i.i unwind label %cleanup.i.i.i.loopexit.i.i.i.i.i.i.i.i.i, !noalias !160813

bb15.i.i.i.i.i.i.i.i.i.i.i.i:                     ; preds = %cleanup.i.i.i.loopexit.i.i.i.i.i.i.i.i.i, %bb14.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i
  %eh.lpad-body.i.i.i.i.i.i.i.i.i.i.i.i = phi { ptr, i32 } [ %.pn.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, %bb14.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i ], [ %lpad.loopexit44.i.i.i.i.i.i.i.i.i, %cleanup.i.i.i.loopexit.i.i.i.i.i.i.i.i.i ]
  %_14.i.i.i.i.i.i.i.i.i.i.i.i = load i64, ptr %residual.i.i.i.i.i.i.i.i.i.i.i.i, align 8, !range !5056, !noalias !160813, !noundef !3995
  %.not.i.i.i.i.i.i.i.i.i = icmp eq i64 %_14.i.i.i.i.i.i.i.i.i.i.i.i, 0
  br i1 %.not.i.i.i.i.i.i.i.i.i, label %bb4.i.i.i.i.i.i.i.i.i.i, label %bb14.i.i.i.i.i.i.i.i.i.i.i.i

cleanup.i.i.i.loopexit.i.i.i.i.i.i.i.i.i:         ; preds = %<alloc::vec::Vec<core::option::Option<purrdf_core::model::RdfTerm>> as core::iter::traits::collect::FromIterator<core::option::Option<purrdf_core::model::RdfTerm>>>::from_iter::<core::iter::adapters::GenericShunt<core::iter::adapters::map::Map<alloc::vec::into_iter::IntoIter<core::option::Option<purrdf_core::ir::term::TermValue>>, purrdf_native::py_store::query::materialize_results::{closure#0}::{closure#0}>, core::result::Result<!, pyo3::err::PyErr>>> (.exit.i.i.i.i.i.i.i.i.i.i.i.i.i)
  %lpad.loopexit44.i.i.i.i.i.i.i.i.i = landingpad { ptr, i32 }
          cleanup
  store ptr %_49.i.i.i.i.i.i.i.i.i, ptr %_8.sroa.4.0._2.i.i.i.sroa_idx, align 8, !alias.scope !160883, !noalias !160884
  br label %bb15.i.i.i.i.i.i.i.i.i.i.i.i

bb15.i.i.i.thread.i.i.i.i.i.i.i.i.i:              ; preds = %bb5.i.i.i.i.i.i.i.i.i.i.i.i
  %lpad.loopexit.split-lp45.i.i.i.i.i.i.i.i.i = landingpad { ptr, i32 }
          cleanup
  br label %bb4.i.i.i.i.i.i.i.i.i.i

bb1.i.i.i.i.i.i.i.i.i.i.i.i:                      ; preds = %<alloc::vec::Vec<core::option::Option<purrdf_core::model::RdfTerm>> as core::iter::traits::collect::FromIterator<core::option::Option<purrdf_core::model::RdfTerm>>>::from_iter::<core::iter::adapters::GenericShunt<core::iter::adapters::map::Map<alloc::vec::into_iter::IntoIter<core::option::Option<purrdf_core::ir::term::TermValue>>, purrdf_native::py_store::query::materialize_results::{closure#0}::{closure#0}>, core::result::Result<!, pyo3::err::PyErr>>> (.exit.i.i.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.end.p0(ptr nonnull %_2.i.i.i.i.i.i.i.i.i.i.i.i.i.i), !noalias !160829
  %_9.i.i.i.i.i.i.i.i.i.i.i.i = load i64, ptr %residual.i.i.i.i.i.i.i.i.i.i.i.i, align 8, !range !5056, !noalias !160813, !noundef !3995
  %64 = trunc nuw i64 %_9.i.i.i.i.i.i.i.i.i.i.i.i to i1
  br i1 %64, label %bb5.i.i.i.i.i.i.i.i.i.i.i.i, label %bb14.i.i.i.i.i.i.i.i.i

bb5.i.i.i.i.i.i.i.i.i.i.i.i:                      ; preds = %bb1.i.i.i.i.i.i.i.i.i.i.i.i
  store ptr %_49.i.i.i.i.i.i.i.i.i, ptr %_8.sroa.4.0._2.i.i.i.sroa_idx, align 8, !alias.scope !160883, !noalias !160884
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %_7.sroa.5.i.i.i.i.i.i.i.i.i.i, ptr noundef nonnull align 8 dereferenceable(48) %_11.sroa.5.0._16.sroa_idx.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, i64 48, i1 false), !noalias !160946
; invoke core::ptr::drop_glue::<alloc::vec::Vec<core::option::Option<purrdf_core::model::RdfTerm>>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<core::option::Option<purrdf_core::model::RdfTerm>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %value.i.i.i.i.i.i.i.i.i.i.i.i)
          to label %bb3.i.i.i.i.i.i.i.i.i.i.i unwind label %bb15.i.i.i.thread.i.i.i.i.i.i.i.i.i, !noalias !160813

terminate.i.i.i.i.i.i.i.i.i.i.i.i:                ; preds = %bb14.i.i.i.i.i.i.i.i.i.i.i.i
  %65 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #75, !noalias !160813
  unreachable

bb14.i.i.i.i.i.i.i.i.i.i.i.i:                     ; preds = %bb15.i.i.i.i.i.i.i.i.i.i.i.i
; invoke core::ptr::drop_glue::<pyo3::err::PyErr>
  invoke void @core::ptr::drop_glue::<pyo3::err::PyErr>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(48) %_11.sroa.5.0._16.sroa_idx.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i)
          to label %bb4.i.i.i.i.i.i.i.i.i.i unwind label %terminate.i.i.i.i.i.i.i.i.i.i.i.i, !noalias !160813

bb3.i.i.i.i.i.i.i.i.i.i.i:                        ; preds = %bb5.i.i.i.i.i.i.i.i.i.i.i.i
  call void @llvm.lifetime.end.p0(ptr nonnull %value.i.i.i.i.i.i.i.i.i.i.i.i), !noalias !160813
  call void @llvm.lifetime.end.p0(ptr nonnull %residual.i.i.i.i.i.i.i.i.i.i.i.i), !noalias !160813
  %_2.i6.i.i.i.i.i.i.i.i.i.i.i = load i64, ptr %residual.i, align 8, !range !5056, !alias.scope !160947, !noalias !160950, !noundef !3995
  %66 = icmp eq i64 %_2.i6.i.i.i.i.i.i.i.i.i.i.i, 0
  br i1 %66, label %bb7.i.i.i.i.i.i.i, label %bb2.i7.i.i.i.i.i.i.i.i.i.i.i

bb2.i7.i.i.i.i.i.i.i.i.i.i.i:                     ; preds = %bb3.i.i.i.i.i.i.i.i.i.i.i
  %67 = getelementptr inbounds nuw i8, ptr %residual.i, i64 8
  call void @llvm.experimental.noalias.scope.decl(metadata !160954)
  call void @llvm.experimental.noalias.scope.decl(metadata !160957), !noalias !160950
  %68 = getelementptr inbounds nuw i8, ptr %residual.i, i64 24
  call void @llvm.experimental.noalias.scope.decl(metadata !160960), !noalias !160950
  call void @llvm.experimental.noalias.scope.decl(metadata !160963), !noalias !160950
  %_2.i.i.i.i = load i64, ptr %68, align 8, !range !5056, !alias.scope !160966, !noalias !160950, !noundef !3995
  %69 = icmp eq i64 %_2.i.i.i.i, 0
  br i1 %69, label %bb7.i.i.i.i.i.i.i, label %bb2.i.i.i.i

bb2.i.i.i.i:                                      ; preds = %bb2.i7.i.i.i.i.i.i.i.i.i.i.i
  %70 = getelementptr inbounds nuw i8, ptr %residual.i, i64 32
  %.val.i.i.i.i = load ptr, ptr %70, align 8, !alias.scope !160966, !noalias !160950, !noundef !3995
  %71 = getelementptr inbounds nuw i8, ptr %residual.i, i64 40
  %.val1.i.i.i.i = load ptr, ptr %71, align 8, !alias.scope !160966, !noalias !160950, !nonnull !3995, !noundef !3995
  %.not.i.i.i.i.i = icmp eq ptr %.val.i.i.i.i, null
  br i1 %.not.i.i.i.i.i, label %bb3.i.i.i.i.i, label %bb2.i.i.i.i.i79

bb2.i.i.i.i.i79:                                  ; preds = %bb2.i.i.i.i
  %72 = load ptr, ptr %.val1.i.i.i.i, align 8, !invariant.load !3995, !noalias !160967
  %.not.i.i.i.i.i.i = icmp eq ptr %72, null
  br i1 %.not.i.i.i.i.i.i, label %bb3.i.i.i.i.i.i, label %is_not_null.i.i.i.i.i.i

is_not_null.i.i.i.i.i.i:                          ; preds = %bb2.i.i.i.i.i79
  invoke void %72(ptr noundef nonnull %.val.i.i.i.i)
          to label %bb3.i.i.i.i.i.i unwind label %cleanup.i.i.i.i.i.i, !noalias !160967

bb3.i.i.i.i.i.i:                                  ; preds = %is_not_null.i.i.i.i.i.i, %bb2.i.i.i.i.i79
  %73 = getelementptr inbounds nuw i8, ptr %.val1.i.i.i.i, i64 8
  %size.i.i.i.i.i.i.i = load i64, ptr %73, align 8, !range !4210, !invariant.load !3995, !noalias !160967
  %74 = icmp eq i64 %size.i.i.i.i.i.i.i, 0
  br i1 %74, label %bb7.i.i.i.i.i.i.i, label %<alloc::alloc::Global as core::alloc::Allocator>::deallocate (.exit.i.i.i.i.i.i.i)

<alloc::alloc::Global as core::alloc::Allocator>::deallocate (.exit.i.i.i.i.i.i.i): ; preds = %bb3.i.i.i.i.i.i
  %75 = getelementptr inbounds nuw i8, ptr %.val1.i.i.i.i, i64 16
  %align.i.i.i.i.i.i.i = load i64, ptr %75, align 8, !range !3997, !invariant.load !3995, !noalias !160967
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %.val.i.i.i.i, i64 noundef %size.i.i.i.i.i.i.i, i64 noundef range(i64 1, -9223372036854775807) %align.i.i.i.i.i.i.i) #77, !noalias !160967
  br label %bb7.i.i.i.i.i.i.i

cleanup.i.i.i.i.i.i:                              ; preds = %is_not_null.i.i.i.i.i.i
  %76 = landingpad { ptr, i32 }
          cleanup
  %77 = getelementptr inbounds nuw i8, ptr %.val1.i.i.i.i, i64 8
  %size.i4.i.i.i.i.i.i = load i64, ptr %77, align 8, !range !4210, !invariant.load !3995, !noalias !160967
  %78 = icmp eq i64 %size.i4.i.i.i.i.i.i, 0
  br i1 %78, label %bb11.i.i.i.i.i.i.i.i.i.i.i.body, label %<alloc::alloc::Global as core::alloc::Allocator>::deallocate (.exit.i5.i.i.i.i.i.i)

<alloc::alloc::Global as core::alloc::Allocator>::deallocate (.exit.i5.i.i.i.i.i.i): ; preds = %cleanup.i.i.i.i.i.i
  %79 = getelementptr inbounds nuw i8, ptr %.val1.i.i.i.i, i64 16
  %align.i6.i.i.i.i.i.i = load i64, ptr %79, align 8, !range !3997, !invariant.load !3995, !noalias !160967
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %.val.i.i.i.i, i64 noundef %size.i4.i.i.i.i.i.i, i64 noundef range(i64 1, -9223372036854775807) %align.i6.i.i.i.i.i.i) #77, !noalias !160967
  br label %bb11.i.i.i.i.i.i.i.i.i.i.i.body

bb3.i.i.i.i.i:                                    ; preds = %bb2.i.i.i.i
  %self3.val.i.i.i.i.i.i.i.i.i.i = load i64, ptr %_3.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, align 8, !noalias !160967, !noundef !3995
  %_0.i.i.i.i.i.i.i.i.i.i.i = icmp sgt i64 %self3.val.i.i.i.i.i.i.i.i.i.i, 0
  br i1 %_0.i.i.i.i.i.i.i.i.i.i.i, label %bb1.i.i.i.i.i.i.i.i.i, label %bb2.i.i.i.i.i.i.i.i.i, !prof !7466

bb2.i.i.i.i.i.i.i.i.i:                            ; preds = %bb3.i.i.i.i.i
; invoke <pyo3::instance::Py<_> as core::ops::drop::Drop>::drop::drop_slow
  invoke void @<pyo3::instance::Py<_> as core::ops::drop::Drop>::drop::drop_slow(ptr noundef nonnull %.val1.i.i.i.i)
          to label %bb7.i.i.i.i.i.i.i unwind label %bb11.i.i.i.i.i.i.i.i.i.i.i

bb1.i.i.i.i.i.i.i.i.i:                            ; preds = %bb3.i.i.i.i.i
  call void @_Py_DecRef(ptr noundef nonnull %.val1.i.i.i.i) #77, !noalias !160967
  br label %bb7.i.i.i.i.i.i.i

bb11.i.i.i.i.i.i.i.i.i.i.i:                       ; preds = %bb2.i.i.i.i.i.i.i.i.i
  %80 = landingpad { ptr, i32 }
          cleanup
  br label %bb11.i.i.i.i.i.i.i.i.i.i.i.body

bb11.i.i.i.i.i.i.i.i.i.i.i.body:                  ; preds = %cleanup.i.i.i.i.i.i, %<alloc::alloc::Global as core::alloc::Allocator>::deallocate (.exit.i5.i.i.i.i.i.i), %bb11.i.i.i.i.i.i.i.i.i.i.i
  %eh.lpad-body80 = phi { ptr, i32 } [ %80, %bb11.i.i.i.i.i.i.i.i.i.i.i ], [ %76, %<alloc::alloc::Global as core::alloc::Allocator>::deallocate (.exit.i5.i.i.i.i.i.i) ], [ %76, %cleanup.i.i.i.i.i.i ]
  store i64 1, ptr %residual.i, align 8, !noalias !160950
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %67, ptr noundef nonnull align 8 dereferenceable(48) %_7.sroa.5.i.i.i.i.i.i.i.i.i.i, i64 48, i1 false), !noalias !160968
; invoke core::ptr::drop_glue::<alloc::vec::in_place_drop::InPlaceDrop<alloc::vec::Vec<core::option::Option<purrdf_core::model::RdfTerm>>>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::in_place_drop::InPlaceDrop<alloc::vec::Vec<core::option::Option<purrdf_core::model::RdfTerm>>>>(ptr nonnull %rows3, ptr nonnull %_46139.i.i.i.i.i.i.i.i.i) #79
          to label %bb14.i.i.i.i.i unwind label %terminate.i.i.i.i.i.i.i.i.i.i.i, !noalias !160950

terminate.i.i.i.i.i.i.i.i.i.i.i:                  ; preds = %bb11.i.i.i.i.i.i.i.i.i.i.i.body
  %81 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #75, !noalias !160950
  unreachable

bb4.i.i.i.i.i.i.i.i.i.i:                          ; preds = %bb14.i.i.i.i.i.i.i.i.i.i.i.i, %bb15.i.i.i.thread.i.i.i.i.i.i.i.i.i, %bb15.i.i.i.i.i.i.i.i.i.i.i.i
  %eh.lpad-body.i.i.i218.i.i.i.i.i.i.i.i.i = phi { ptr, i32 } [ %lpad.loopexit.split-lp45.i.i.i.i.i.i.i.i.i, %bb15.i.i.i.thread.i.i.i.i.i.i.i.i.i ], [ %eh.lpad-body.i.i.i.i.i.i.i.i.i.i.i.i, %bb14.i.i.i.i.i.i.i.i.i.i.i.i ], [ %eh.lpad-body.i.i.i.i.i.i.i.i.i.i.i.i, %bb15.i.i.i.i.i.i.i.i.i.i.i.i ]
; invoke core::ptr::drop_glue::<alloc::vec::in_place_drop::InPlaceDrop<alloc::vec::Vec<core::option::Option<purrdf_core::model::RdfTerm>>>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::in_place_drop::InPlaceDrop<alloc::vec::Vec<core::option::Option<purrdf_core::model::RdfTerm>>>>(ptr nonnull %rows3, ptr nonnull %_46139.i.i.i.i.i.i.i.i.i) #79
          to label %bb14.i.i.i.i.i unwind label %terminate.i.i.i.i.i.i.i.i.i.i, !noalias !160968

terminate.i.i.i.i.i.i.i.i.i.i:                    ; preds = %bb4.i.i.i.i.i.i.i.i.i.i
  %82 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #75, !noalias !160968
  unreachable

bb14.i.i.i.i.i.i.i.i.i:                           ; preds = %bb1.i.i.i.i.i.i.i.i.i.i.i.i
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %_7.sroa.5.i.i.i.i.i.i.i.i.i.i, ptr noundef nonnull align 8 dereferenceable(24) %value.i.i.i.i.i.i.i.i.i.i.i.i, i64 24, i1 false), !noalias !160946
  call void @llvm.lifetime.end.p0(ptr nonnull %value.i.i.i.i.i.i.i.i.i.i.i.i), !noalias !160813
  call void @llvm.lifetime.end.p0(ptr nonnull %residual.i.i.i.i.i.i.i.i.i.i.i.i), !noalias !160813
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %_46139.i.i.i.i.i.i.i.i.i, ptr noundef nonnull align 8 dereferenceable(24) %_7.sroa.5.i.i.i.i.i.i.i.i.i.i, i64 24, i1 false), !noalias !160968
  call void @llvm.lifetime.end.p0(ptr nonnull %_7.sroa.5.i.i.i.i.i.i.i.i.i.i)
  %_44.not.i.i.i.i.i.i.i.i.i = icmp eq ptr %_49.i.i.i.i.i.i.i.i.i, %_38
  br i1 %_44.not.i.i.i.i.i.i.i.i.i, label %bb6.i.i.i.i.i, label %bb11.i.i.i.i.i.i.i.i.i

bb7.i.i.i.i.i.i.i:                                ; preds = %bb1.i.i.i.i.i.i.i.i.i, %<alloc::alloc::Global as core::alloc::Allocator>::deallocate (.exit.i.i.i.i.i.i.i), %bb3.i.i.i.i.i.i, %bb2.i7.i.i.i.i.i.i.i.i.i.i.i, %bb2.i.i.i.i.i.i.i.i.i, %bb3.i.i.i.i.i.i.i.i.i.i.i
  store i64 1, ptr %residual.i, align 8, !noalias !160950
  %_11.sroa.5.0._16.sroa_idx.i.i.i.i.i.i.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %residual.i, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %_11.sroa.5.0._16.sroa_idx.i.i.i.i.i.i.i.i.i.i.i, ptr noundef nonnull align 8 dereferenceable(48) %_7.sroa.5.i.i.i.i.i.i.i.i.i.i, i64 48, i1 false), !noalias !160968
  call void @llvm.lifetime.end.p0(ptr nonnull %_7.sroa.5.i.i.i.i.i.i.i.i.i.i)
  br label %bb6.i.i.i.i.i

bb6.i.i.i.i.i:                                    ; preds = %bb14.i.i.i.i.i.i.i.i.i, %bb7.i.i.i.i.i.i.i, %bb4
  %self.val.i.i.i.i.i.i = phi ptr [ %_49.i.i.i.i.i.i.i.i.i, %bb7.i.i.i.i.i.i.i ], [ %rows3, %bb4 ], [ %_38, %bb14.i.i.i.i.i.i.i.i.i ]
  %accum.sroa.4.0138.i.i.pn.i.i.i.i.i.i.i = phi ptr [ %_46139.i.i.i.i.i.i.i.i.i, %bb7.i.i.i.i.i.i.i ], [ %rows3, %bb4 ], [ %_38, %bb14.i.i.i.i.i.i.i.i.i ]
  %83 = ptrtoint ptr %accum.sroa.4.0138.i.i.pn.i.i.i.i.i.i.i to i64
  %84 = ptrtoint ptr %rows3 to i64
  %85 = sub nuw i64 %83, %84
  %_0.i.i.i.i.i.i = udiv exact i64 %85, 24
  call void @llvm.lifetime.start.p0(ptr nonnull %dst_guard.i.i.i.i.i), !noalias !160969
  store ptr %rows3, ptr %dst_guard.i.i.i.i.i, align 8, !noalias !160969
  %86 = getelementptr inbounds nuw i8, ptr %dst_guard.i.i.i.i.i, i64 8
  store i64 %_0.i.i.i.i.i.i, ptr %86, align 8, !noalias !160969
  %87 = getelementptr inbounds nuw i8, ptr %dst_guard.i.i.i.i.i, i64 16
  store i64 %rows4, ptr %87, align 8, !noalias !160969
  call void @llvm.experimental.noalias.scope.decl(metadata !160970)
  %88 = ptrtoint ptr %_38 to i64
  %89 = ptrtoint ptr %self.val.i.i.i.i.i.i to i64
  %90 = sub nuw i64 %88, %89
  %91 = udiv exact i64 %90, 24
  store i64 0, ptr %_8.sroa.5.0._2.i.i.i.sroa_idx, align 8, !alias.scope !160973, !noalias !160807
  store ptr inttoptr (i64 8 to ptr), ptr %_2.i.i.i, align 8, !alias.scope !160973, !noalias !160807
  store ptr inttoptr (i64 8 to ptr), ptr %_8.sroa.4.0._2.i.i.i.sroa_idx, align 8, !alias.scope !160973, !noalias !160807
  store ptr inttoptr (i64 8 to ptr), ptr %_8.sroa.6.0._2.i.i.i.sroa_idx, align 8, !alias.scope !160973, !noalias !160807
  %_7.i.i.i.i.i.i.i461 = icmp eq ptr %_38, %self.val.i.i.i.i.i.i
  br i1 %_7.i.i.i.i.i.i.i461, label %<alloc::vec::Vec<alloc::vec::Vec<core::option::Option<purrdf_core::model::RdfTerm>>> as core::iter::traits::collect::FromIterator<alloc::vec::Vec<core::option::Option<purrdf_core::model::RdfTerm>>>>::from_iter::<core::iter::adapters::GenericShunt<core::iter::adapters::map::Map<alloc::vec::into_iter::IntoIter<alloc::vec::Vec<core::option::Option<purrdf_core::ir::term::TermValue>>>, purrdf_native::py_store::query::materialize_results::{closure#0}>, core::result::Result<!, pyo3::err::PyErr>>> (.exit.i.i), label %bb5.i.i.i.i.i.i.i

bb6.i.i.i.i.i.i.i:                                ; preds = %bb5.i.i.i.i.i.i.i
  %_7.i.i.i.i.i.i.i = icmp eq i64 %92, %91
  br i1 %_7.i.i.i.i.i.i.i, label %<alloc::vec::Vec<alloc::vec::Vec<core::option::Option<purrdf_core::model::RdfTerm>>> as core::iter::traits::collect::FromIterator<alloc::vec::Vec<core::option::Option<purrdf_core::model::RdfTerm>>>>::from_iter::<core::iter::adapters::GenericShunt<core::iter::adapters::map::Map<alloc::vec::into_iter::IntoIter<alloc::vec::Vec<core::option::Option<purrdf_core::ir::term::TermValue>>>, purrdf_native::py_store::query::materialize_results::{closure#0}>, core::result::Result<!, pyo3::err::PyErr>>> (.exit.i.i), label %bb5.i.i.i.i.i.i.i

bb5.i.i.i.i.i.i.i:                                ; preds = %bb6.i.i.i.i.i, %bb6.i.i.i.i.i.i.i
  %_3.sroa.0.0.i.i.i.i.i.i.i462 = phi i64 [ %92, %bb6.i.i.i.i.i.i.i ], [ 0, %bb6.i.i.i.i.i ]
  %_6.i.i.i.i.i.i.i = getelementptr inbounds nuw [24 x i8], ptr %self.val.i.i.i.i.i.i, i64 %_3.sroa.0.0.i.i.i.i.i.i.i462
  %92 = add nuw nsw i64 %_3.sroa.0.0.i.i.i.i.i.i.i462, 1
; invoke core::ptr::drop_glue::<alloc::vec::Vec<core::option::Option<purrdf_core::ir::term::TermValue>>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<core::option::Option<purrdf_core::ir::term::TermValue>>>(ptr noalias nofree noundef readonly align 8 dereferenceable(24) %_6.i.i.i.i.i.i.i)
          to label %bb6.i.i.i.i.i.i.i unwind label %cleanup.i.i.i.i.i.i.i, !noalias !160974

bb4.i.i.i.i.i.i.i:                                ; preds = %bb3.i.i.i.i.i.i.i
  %93 = add i64 %_3.sroa.0.1.i.i.i.i.i.i.i464, 1
  %_5.i.i.i.i.i.i.i = icmp eq i64 %93, %91
  br i1 %_5.i.i.i.i.i.i.i, label %cleanup3.body.i.i.i.i.i, label %bb3.i.i.i.i.i.i.i

cleanup.i.i.i.i.i.i.i:                            ; preds = %bb5.i.i.i.i.i.i.i
  %94 = landingpad { ptr, i32 }
          cleanup
  %_5.i.i.i.i.i.i.i463 = icmp eq i64 %92, %91
  br i1 %_5.i.i.i.i.i.i.i463, label %cleanup3.body.i.i.i.i.i, label %bb3.i.i.i.i.i.i.i

bb3.i.i.i.i.i.i.i:                                ; preds = %cleanup.i.i.i.i.i.i.i, %bb4.i.i.i.i.i.i.i
  %_3.sroa.0.1.i.i.i.i.i.i.i464 = phi i64 [ %93, %bb4.i.i.i.i.i.i.i ], [ %92, %cleanup.i.i.i.i.i.i.i ]
  %_4.i.i.i.i.i.i.i = getelementptr inbounds nuw [24 x i8], ptr %self.val.i.i.i.i.i.i, i64 %_3.sroa.0.1.i.i.i.i.i.i.i464
; invoke core::ptr::drop_glue::<alloc::vec::Vec<core::option::Option<purrdf_core::ir::term::TermValue>>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<core::option::Option<purrdf_core::ir::term::TermValue>>>(ptr noalias nofree noundef readonly align 8 dereferenceable(24) %_4.i.i.i.i.i.i.i) #79
          to label %bb4.i.i.i.i.i.i.i unwind label %terminate.i.i.i.i.i.i.i, !noalias !160974

terminate.i.i.i.i.i.i.i:                          ; preds = %bb3.i.i.i.i.i.i.i
  %95 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #75, !noalias !160975
  unreachable

cleanup3.body.i.i.i.i.i:                          ; preds = %bb4.i.i.i.i.i.i.i, %cleanup.i.i.i.i.i.i.i
; invoke core::ptr::drop_glue::<alloc::vec::in_place_drop::InPlaceDstDataSrcBufDrop<alloc::vec::Vec<core::option::Option<purrdf_core::ir::term::TermValue>>, alloc::vec::Vec<core::option::Option<purrdf_core::model::RdfTerm>>>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::in_place_drop::InPlaceDstDataSrcBufDrop<alloc::vec::Vec<core::option::Option<purrdf_core::ir::term::TermValue>>, alloc::vec::Vec<core::option::Option<purrdf_core::model::RdfTerm>>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %dst_guard.i.i.i.i.i) #79
          to label %bb14.i.i.i.i.i unwind label %terminate.i.i.i.i.i, !noalias !160969

terminate.i.i.i.i.i:                              ; preds = %cleanup3.body.i.i.i.i.i, %bb14.i.i.i.i.i
  %96 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #75, !noalias !160969
  unreachable

<alloc::vec::Vec<alloc::vec::Vec<core::option::Option<purrdf_core::model::RdfTerm>>> as core::iter::traits::collect::FromIterator<alloc::vec::Vec<core::option::Option<purrdf_core::model::RdfTerm>>>>::from_iter::<core::iter::adapters::GenericShunt<core::iter::adapters::map::Map<alloc::vec::into_iter::IntoIter<alloc::vec::Vec<core::option::Option<purrdf_core::ir::term::TermValue>>>, purrdf_native::py_store::query::materialize_results::{closure#0}>, core::result::Result<!, pyo3::err::PyErr>>> (.exit.i.i): ; preds = %bb6.i.i.i.i.i.i.i, %bb6.i.i.i.i.i
  call void @llvm.lifetime.end.p0(ptr nonnull %dst_guard.i.i.i.i.i), !noalias !160969
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<alloc::vec::Vec<core::option::Option<purrdf_core::ir::term::TermValue>>>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<alloc::vec::Vec<core::option::Option<purrdf_core::ir::term::TermValue>>>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(40) %_2.i.i.i)
          to label %bb1.i unwind label %cleanup.i, !noalias !160772

bb15.i:                                           ; preds = %cleanup.i, %bb14.i.i.i.i.i
  %eh.lpad-body.i = phi { ptr, i32 } [ %.pn.i.i.i.i.i, %bb14.i.i.i.i.i ], [ %97, %cleanup.i ]
  %_14.i = load i64, ptr %residual.i, align 8, !range !5056, !noalias !160772, !noundef !3995
  %.not.i = icmp eq i64 %_14.i, 0
  br i1 %.not.i, label %bb19, label %bb14.i

cleanup.i:                                        ; preds = %<alloc::vec::Vec<alloc::vec::Vec<core::option::Option<purrdf_core::model::RdfTerm>>> as core::iter::traits::collect::FromIterator<alloc::vec::Vec<core::option::Option<purrdf_core::model::RdfTerm>>>>::from_iter::<core::iter::adapters::GenericShunt<core::iter::adapters::map::Map<alloc::vec::into_iter::IntoIter<alloc::vec::Vec<core::option::Option<purrdf_core::ir::term::TermValue>>>, purrdf_native::py_store::query::materialize_results::{closure#0}>, core::result::Result<!, pyo3::err::PyErr>>> (.exit.i.i)
  %97 = landingpad { ptr, i32 }
          cleanup
  br label %bb15.i

bb1.i:                                            ; preds = %<alloc::vec::Vec<alloc::vec::Vec<core::option::Option<purrdf_core::model::RdfTerm>>> as core::iter::traits::collect::FromIterator<alloc::vec::Vec<core::option::Option<purrdf_core::model::RdfTerm>>>>::from_iter::<core::iter::adapters::GenericShunt<core::iter::adapters::map::Map<alloc::vec::into_iter::IntoIter<alloc::vec::Vec<core::option::Option<purrdf_core::ir::term::TermValue>>>, purrdf_native::py_store::query::materialize_results::{closure#0}>, core::result::Result<!, pyo3::err::PyErr>>> (.exit.i.i)
  call void @llvm.lifetime.end.p0(ptr nonnull %_2.i.i.i), !noalias !160776
  %_9.i = load i64, ptr %residual.i, align 8, !range !5056, !noalias !160772, !noundef !3995
  %98 = trunc nuw i64 %_9.i to i1
  br i1 %98, label %bb5.i, label %bb25

bb5.i:                                            ; preds = %bb1.i
  %99 = getelementptr inbounds nuw i8, ptr %residual.i, i64 8
  %_7.sroa.6.8.copyload82 = load i64, ptr %99, align 8, !noalias !160978
  %_7.sroa.10.8..sroa_idx = getelementptr inbounds nuw i8, ptr %residual.i, i64 16
  %_7.sroa.10.8.copyload84 = load ptr, ptr %_7.sroa.10.8..sroa_idx, align 8, !noalias !160978
  %_7.sroa.11.8..sroa_idx = getelementptr inbounds nuw i8, ptr %residual.i, i64 24
  %_7.sroa.11.8.copyload86 = load i64, ptr %_7.sroa.11.8..sroa_idx, align 8, !noalias !160978
  %_7.sroa.12.8..sroa_idx = getelementptr inbounds nuw i8, ptr %residual.i, i64 32
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %_7.sroa.12, ptr noundef nonnull align 8 dereferenceable(24) %_7.sroa.12.8..sroa_idx, i64 24, i1 false), !noalias !160978
  %_7.i.i.i.i465 = icmp eq ptr %accum.sroa.4.0138.i.i.pn.i.i.i.i.i.i.i, %rows3
  br i1 %_7.i.i.i.i465, label %bb4.i.i, label %bb5.i.i.i.i

bb6.i.i.i.i:                                      ; preds = %bb5.i.i.i.i
  %_7.i.i.i.i = icmp eq i64 %100, %_0.i.i.i.i.i.i
  br i1 %_7.i.i.i.i, label %bb4.i.i, label %bb5.i.i.i.i

bb5.i.i.i.i:                                      ; preds = %bb5.i, %bb6.i.i.i.i
  %_3.sroa.0.0.i.i.i.i466 = phi i64 [ %100, %bb6.i.i.i.i ], [ 0, %bb5.i ]
  %_6.i.i.i.i = getelementptr inbounds nuw [24 x i8], ptr %rows3, i64 %_3.sroa.0.0.i.i.i.i466
  %100 = add nuw nsw i64 %_3.sroa.0.0.i.i.i.i466, 1
; invoke core::ptr::drop_glue::<alloc::vec::Vec<core::option::Option<purrdf_core::model::RdfTerm>>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<core::option::Option<purrdf_core::model::RdfTerm>>>(ptr noalias nofree noundef readonly align 8 dereferenceable(24) %_6.i.i.i.i)
          to label %bb6.i.i.i.i unwind label %cleanup.i.i.i.i, !noalias !160979

bb4.i.i.i.i:                                      ; preds = %bb3.i.i.i.i
  %101 = add i64 %_3.sroa.0.1.i.i.i.i468, 1
  %_5.i.i.i.i = icmp eq i64 %101, %_0.i.i.i.i.i.i
  br i1 %_5.i.i.i.i, label %cleanup.body.i.i, label %bb3.i.i.i.i

cleanup.i.i.i.i:                                  ; preds = %bb5.i.i.i.i
  %102 = landingpad { ptr, i32 }
          cleanup
  %_5.i.i.i.i467 = icmp eq i64 %100, %_0.i.i.i.i.i.i
  br i1 %_5.i.i.i.i467, label %cleanup.body.i.i, label %bb3.i.i.i.i

bb3.i.i.i.i:                                      ; preds = %cleanup.i.i.i.i, %bb4.i.i.i.i
  %_3.sroa.0.1.i.i.i.i468 = phi i64 [ %101, %bb4.i.i.i.i ], [ %100, %cleanup.i.i.i.i ]
  %_4.i.i.i.i = getelementptr inbounds nuw [24 x i8], ptr %rows3, i64 %_3.sroa.0.1.i.i.i.i468
; invoke core::ptr::drop_glue::<alloc::vec::Vec<core::option::Option<purrdf_core::model::RdfTerm>>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<core::option::Option<purrdf_core::model::RdfTerm>>>(ptr noalias nofree noundef readonly align 8 dereferenceable(24) %_4.i.i.i.i) #79
          to label %bb4.i.i.i.i unwind label %terminate.i.i.i.i, !noalias !160979

terminate.i.i.i.i:                                ; preds = %bb3.i.i.i.i
  %103 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #75, !noalias !160982
  unreachable

cleanup.body.i.i:                                 ; preds = %bb4.i.i.i.i, %cleanup.i.i.i.i
  %104 = icmp eq i64 %rows4, 0
  br i1 %104, label %bb19, label %bb2.i.i.i.i.i

bb2.i.i.i.i.i:                                    ; preds = %cleanup.body.i.i
  %alloc_size.i.i.i.i.i.i = mul nuw i64 %rows4, 24
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %rows3, i64 noundef %alloc_size.i.i.i.i.i.i, i64 noundef range(i64 1, -9223372036854775807) 8) #77, !noalias !160979
  br label %bb19

bb4.i.i:                                          ; preds = %bb6.i.i.i.i, %bb5.i
  %105 = icmp eq i64 %rows4, 0
  br i1 %105, label %bb24, label %bb2.i.i.i6.i.i

bb2.i.i.i6.i.i:                                   ; preds = %bb4.i.i
  %alloc_size.i.i.i.i7.i.i = mul nuw i64 %rows4, 24
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %rows3, i64 noundef %alloc_size.i.i.i.i7.i.i, i64 noundef range(i64 1, -9223372036854775807) 8) #77, !noalias !160979
  br label %bb24

terminate.i:                                      ; preds = %bb14.i
  %106 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #75, !noalias !160772
  unreachable

bb14.i:                                           ; preds = %bb15.i
  %107 = getelementptr inbounds nuw i8, ptr %residual.i, i64 8
; invoke core::ptr::drop_glue::<pyo3::err::PyErr>
  invoke void @core::ptr::drop_glue::<pyo3::err::PyErr>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(48) %107)
          to label %bb19 unwind label %terminate.i, !noalias !160772

bb3:                                              ; preds = %start
  call void @llvm.lifetime.start.p0(ptr nonnull %graph)
  %108 = getelementptr inbounds nuw i8, ptr %result, i64 8
  %109 = load ptr, ptr %108, align 8, !nonnull !3995, !noundef !3995
  store ptr %109, ptr %graph, align 8
  %_21 = getelementptr inbounds nuw i8, ptr %109, i64 16
  tail call void @llvm.experimental.noalias.scope.decl(metadata !160985)
  call void @llvm.lifetime.start.p0(ptr nonnull %quads.i), !noalias !160985
; invoke purrdf_rdf::native_quads::flat_rdf_quads_from_dataset
  invoke void @purrdf_rdf::native_quads::flat_rdf_quads_from_dataset(ptr noalias nofree noundef nonnull sret([24 x i8]) align 8 captures(address) dereferenceable(24) %quads.i, ptr noundef nonnull align 8 %_21)
          to label %.noexc unwind label %cleanup9

.noexc:                                           ; preds = %bb3
  %110 = getelementptr inbounds nuw i8, ptr %quads.i, i64 8
  %_25.i = load ptr, ptr %110, align 8, !noalias !160985, !nonnull !3995, !noundef !3995
  %111 = getelementptr inbounds nuw i8, ptr %quads.i, i64 16
  %_24.i = load i64, ptr %111, align 8, !noalias !160985, !noundef !3995
  %_29.idx.i = mul nuw nsw i64 %_24.i, 440
  %_29.i = getelementptr inbounds nuw i8, ptr %_25.i, i64 %_29.idx.i
  %_12.not.not.not.i.not.i455 = icmp eq i64 %_24.i, 0
  br i1 %_12.not.not.not.i.not.i455, label %bb6.i19, label %bb13.i.i

bb1.i.i:                                          ; preds = %bb13.i.i
  %_22.i.i = getelementptr inbounds nuw i8, ptr %_221.i.i456, i64 440
  %_12.not.not.not.i.not.i = icmp eq ptr %_22.i.i, %_29.i
  br i1 %_12.not.not.not.i.not.i, label %bb6.i19, label %bb13.i.i

bb13.i.i:                                         ; preds = %.noexc, %bb1.i.i
  %_221.i.i456 = phi ptr [ %_22.i.i, %bb1.i.i ], [ %_25.i, %.noexc ]
  %112 = getelementptr i8, ptr %_221.i.i456, i64 360
  %ptr.val.i.i = load i64, ptr %112, align 8, !range !4141, !noalias !160988, !noundef !3995
  %.not.i.i = icmp eq i64 %ptr.val.i.i, -1
  br i1 %.not.i.i, label %bb1.i.i, label %bb3.i

bb6.i19:                                          ; preds = %bb1.i.i, %.noexc
  call void @llvm.lifetime.start.p0(ptr nonnull %_14.i16), !noalias !160985
  %_59.i = load i64, ptr %quads.i, align 8, !range !4210, !noalias !160985, !noundef !3995
  %_46.i = icmp ult i64 %_24.i, 20962209174669946
  call void @llvm.assume(i1 %_46.i)
  store ptr %_25.i, ptr %_14.i16, align 8, !noalias !160985
  %_15.sroa.4.0._14.sroa_idx.i = getelementptr inbounds nuw i8, ptr %_14.i16, i64 8
  %_15.sroa.5.0._14.sroa_idx.i = getelementptr inbounds nuw i8, ptr %_14.i16, i64 16
  store i64 %_59.i, ptr %_15.sroa.5.0._14.sroa_idx.i, align 8, !noalias !160985
  %_15.sroa.6.0._14.sroa_idx.i = getelementptr inbounds nuw i8, ptr %_14.i16, i64 24
  store ptr %_29.i, ptr %_15.sroa.6.0._14.sroa_idx.i, align 8, !noalias !160985
  call void @llvm.experimental.noalias.scope.decl(metadata !160991)
  %_12.i.i = mul i64 %_59.i, 440
  %dst_cap.i.i = udiv i64 %_12.i.i, 360
  call void @llvm.experimental.noalias.scope.decl(metadata !160994)
  call void @llvm.experimental.noalias.scope.decl(metadata !160997)
  call void @llvm.experimental.noalias.scope.decl(metadata !161000)
  %_44.not5.i.i.i.i.i = icmp eq i64 %_24.i, 0
  br i1 %_44.not5.i.i.i.i.i, label %bb6.i.i, label %bb11.lr.ph.i.i.i.i.i

bb14.i.i:                                         ; preds = %cleanup3.body.i.i, %bb4.i.i.i.i.i.i
  %.pn.i.i = phi { ptr, i32 } [ %eh.lpad-body9.i.i, %cleanup3.body.i.i ], [ %138, %bb4.i.i.i.i.i.i ]
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::model::RdfQuad>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::model::RdfQuad>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(32) %_14.i16)
          to label %cleanup9.body unwind label %terminate.i.i, !noalias !161003

bb11.lr.ph.i.i.i.i.i:                             ; preds = %bb6.i19
  %113 = getelementptr inbounds nuw i8, ptr %_34.i.i.i.i.i, i64 8
  %114 = getelementptr inbounds nuw i8, ptr %_34.i.i.i.i.i, i64 16
  %_7.sroa.4.176..sroa_idx.i.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %_7.sroa.4.i.i.i.i.i.i, i64 168
  %115 = getelementptr inbounds nuw i8, ptr %_34.i.i.i.i.i, i64 376
  %116 = getelementptr inbounds nuw i8, ptr %_34.i.i.i.i.i, i64 112
  %117 = getelementptr inbounds nuw i8, ptr %_34.i.i.i.i.i, i64 120
  %118 = getelementptr inbounds nuw i8, ptr %_34.i.i.i.i.i, i64 136
  %119 = getelementptr inbounds nuw i8, ptr %_34.i.i.i.i.i, i64 144
  %120 = getelementptr inbounds nuw i8, ptr %_34.i.i.i.i.i, i64 160
  %121 = getelementptr inbounds nuw i8, ptr %_34.i.i.i.i.i, i64 168
  %122 = getelementptr inbounds nuw i8, ptr %_34.i.i.i.i.i, i64 296
  %123 = getelementptr inbounds nuw i8, ptr %_34.i.i.i.i.i, i64 216
  br label %bb11.i.i.i.i.i

bb11.i.i.i.i.i:                                   ; preds = %core::iter::adapters::map::map_try_fold::<purrdf_core::model::RdfQuad, purrdf_core::model::RdfTriple, alloc::vec::in_place_drop::InPlaceDrop<purrdf_core::model::RdfTriple>, core::result::Result<alloc::vec::in_place_drop::InPlaceDrop<purrdf_core::model::RdfTriple>, !>, purrdf_native::py_store::query::materialize_graph::{closure#1}, alloc::vec::in_place_collect::write_in_place_with_drop<purrdf_core::model::RdfTriple>::{closure#0}>::{closure#0} (.exit.i.i.i.i.i), %bb11.lr.ph.i.i.i.i.i
  %accum.sroa.4.07.i.i.i.i.i = phi ptr [ %_25.i, %bb11.lr.ph.i.i.i.i.i ], [ %_5.i.i.i.i.i.i.i21, %core::iter::adapters::map::map_try_fold::<purrdf_core::model::RdfQuad, purrdf_core::model::RdfTriple, alloc::vec::in_place_drop::InPlaceDrop<purrdf_core::model::RdfTriple>, core::result::Result<alloc::vec::in_place_drop::InPlaceDrop<purrdf_core::model::RdfTriple>, !>, purrdf_native::py_store::query::materialize_graph::{closure#1}, alloc::vec::in_place_collect::write_in_place_with_drop<purrdf_core::model::RdfTriple>::{closure#0}>::{closure#0} (.exit.i.i.i.i.i) ]
  %_4946.i.i.i.i.i = phi ptr [ %_25.i, %bb11.lr.ph.i.i.i.i.i ], [ %_49.i.i.i.i.i, %core::iter::adapters::map::map_try_fold::<purrdf_core::model::RdfQuad, purrdf_core::model::RdfTriple, alloc::vec::in_place_drop::InPlaceDrop<purrdf_core::model::RdfTriple>, core::result::Result<alloc::vec::in_place_drop::InPlaceDrop<purrdf_core::model::RdfTriple>, !>, purrdf_native::py_store::query::materialize_graph::{closure#1}, alloc::vec::in_place_collect::write_in_place_with_drop<purrdf_core::model::RdfTriple>::{closure#0}>::{closure#0} (.exit.i.i.i.i.i) ]
  call void @llvm.lifetime.start.p0(ptr nonnull %_34.i.i.i.i.i), !noalias !161005
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(440) %114, ptr noundef nonnull align 8 dereferenceable(440) %_4946.i.i.i.i.i, i64 440, i1 false), !noalias !161005
  %_49.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %_4946.i.i.i.i.i, i64 440
  store ptr %_25.i, ptr %_34.i.i.i.i.i, align 8, !noalias !161005
  store ptr %accum.sroa.4.07.i.i.i.i.i, ptr %113, align 8, !noalias !161005
  call void @llvm.experimental.noalias.scope.decl(metadata !161006)
  call void @llvm.lifetime.start.p0(ptr nonnull %_7.sroa.4.i.i.i.i.i.i)
  call void @llvm.experimental.noalias.scope.decl(metadata !161009)
  %124 = getelementptr inbounds nuw i8, ptr %_4946.i.i.i.i.i, i64 176
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %_7.sroa.4.176..sroa_idx.i.i.i.i.i.i, ptr noundef nonnull align 8 dereferenceable(24) %124, i64 24, i1 false), !noalias !161005
  %125 = load i64, ptr %115, align 8, !range !4141, !alias.scope !161012, !noalias !161015, !noundef !3995
  %126 = icmp eq i64 %125, -1
  br i1 %126, label %bb5.i.i.i.i.i.i.i20, label %bb2.i3.i.i.i.i.i.i.i

bb2.i3.i.i.i.i.i.i.i:                             ; preds = %bb11.i.i.i.i.i
; invoke core::ptr::drop_glue::<purrdf_core::model::RdfTerm>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_core::model::RdfTerm>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(80) %115)
          to label %bb5.i.i.i.i.i.i.i20 unwind label %bb4.i.i.i.i.i.i, !noalias !161015

bb5.i.i.i.i.i.i.i20:                              ; preds = %bb2.i3.i.i.i.i.i.i.i, %bb11.i.i.i.i.i
  call void @llvm.experimental.noalias.scope.decl(metadata !161017)
  %127 = load i64, ptr %114, align 8, !range !4057, !alias.scope !161020, !noalias !161015, !noundef !3995
  %128 = icmp eq i64 %127, 2
  br i1 %128, label %core::iter::adapters::map::map_try_fold::<purrdf_core::model::RdfQuad, purrdf_core::model::RdfTriple, alloc::vec::in_place_drop::InPlaceDrop<purrdf_core::model::RdfTriple>, core::result::Result<alloc::vec::in_place_drop::InPlaceDrop<purrdf_core::model::RdfTriple>, !>, purrdf_native::py_store::query::materialize_graph::{closure#1}, alloc::vec::in_place_collect::write_in_place_with_drop<purrdf_core::model::RdfTriple>::{closure#0}>::{closure#0} (.exit.i.i.i.i.i), label %bb2.i6.i.i.i.i.i.i.i

bb2.i6.i.i.i.i.i.i.i:                             ; preds = %bb5.i.i.i.i.i.i.i20
  call void @llvm.experimental.noalias.scope.decl(metadata !161021)
  call void @llvm.experimental.noalias.scope.decl(metadata !161024)
  %129 = load i64, ptr %116, align 8, !range !4012, !alias.scope !161027, !noalias !161015, !noundef !3995
  %130 = icmp eq i64 %129, -1
  br i1 %130, label %bb6.i.i.i.i.i.i.i.i.i, label %bb2.i.i.i.i.i.i.i.i.i.i

bb2.i.i.i.i.i.i.i.i.i.i:                          ; preds = %bb2.i6.i.i.i.i.i.i.i
  call void @llvm.experimental.noalias.scope.decl(metadata !161028)
  %131 = icmp eq i64 %129, 0
  br i1 %131, label %bb6.i.i.i.i.i.i.i.i.i, label %bb2.i.i.i4.i.i.i.i.i.i.i.i.i.i.i.i

bb2.i.i.i4.i.i.i.i.i.i.i.i.i.i.i.i:               ; preds = %bb2.i.i.i.i.i.i.i.i.i.i
  %_1.val1.i.i.i.i.i.i.i.i.i.i.i = load ptr, ptr %117, align 8, !alias.scope !161031, !noalias !161015, !nonnull !3995, !noundef !3995
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %_1.val1.i.i.i.i.i.i.i.i.i.i.i, i64 noundef %129, i64 noundef range(i64 1, -9223372036854775807) 1) #77, !noalias !161032
  br label %bb6.i.i.i.i.i.i.i.i.i

bb6.i.i.i.i.i.i.i.i.i:                            ; preds = %bb2.i.i.i4.i.i.i.i.i.i.i.i.i.i.i.i, %bb2.i.i.i.i.i.i.i.i.i.i, %bb2.i6.i.i.i.i.i.i.i
  call void @llvm.experimental.noalias.scope.decl(metadata !161033)
  %132 = load i64, ptr %118, align 8, !range !4012, !alias.scope !161036, !noalias !161015, !noundef !3995
  %133 = icmp eq i64 %132, -1
  br i1 %133, label %bb5.i.i.i.i.i.i.i.i.i, label %bb2.i7.i.i.i.i.i.i.i.i.i

bb2.i7.i.i.i.i.i.i.i.i.i:                         ; preds = %bb6.i.i.i.i.i.i.i.i.i
  call void @llvm.experimental.noalias.scope.decl(metadata !161037)
  %134 = icmp eq i64 %132, 0
  br i1 %134, label %bb5.i.i.i.i.i.i.i.i.i, label %bb2.i.i.i4.i.i.i8.i.i.i.i.i.i.i.i.i

bb2.i.i.i4.i.i.i8.i.i.i.i.i.i.i.i.i:              ; preds = %bb2.i7.i.i.i.i.i.i.i.i.i
  %_1.val1.i.i9.i.i.i.i.i.i.i.i.i = load ptr, ptr %119, align 8, !alias.scope !161040, !noalias !161015, !nonnull !3995, !noundef !3995
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %_1.val1.i.i9.i.i.i.i.i.i.i.i.i, i64 noundef %132, i64 noundef range(i64 1, -9223372036854775807) 1) #77, !noalias !161041
  br label %bb5.i.i.i.i.i.i.i.i.i

bb5.i.i.i.i.i.i.i.i.i:                            ; preds = %bb2.i.i.i4.i.i.i8.i.i.i.i.i.i.i.i.i, %bb2.i7.i.i.i.i.i.i.i.i.i, %bb6.i.i.i.i.i.i.i.i.i
  call void @llvm.experimental.noalias.scope.decl(metadata !161042)
  %135 = load i64, ptr %120, align 8, !range !4012, !alias.scope !161045, !noalias !161015, !noundef !3995
  %136 = icmp eq i64 %135, -1
  br i1 %136, label %core::iter::adapters::map::map_try_fold::<purrdf_core::model::RdfQuad, purrdf_core::model::RdfTriple, alloc::vec::in_place_drop::InPlaceDrop<purrdf_core::model::RdfTriple>, core::result::Result<alloc::vec::in_place_drop::InPlaceDrop<purrdf_core::model::RdfTriple>, !>, purrdf_native::py_store::query::materialize_graph::{closure#1}, alloc::vec::in_place_collect::write_in_place_with_drop<purrdf_core::model::RdfTriple>::{closure#0}>::{closure#0} (.exit.i.i.i.i.i), label %bb2.i15.i.i.i.i.i.i.i.i.i

bb2.i15.i.i.i.i.i.i.i.i.i:                        ; preds = %bb5.i.i.i.i.i.i.i.i.i
  call void @llvm.experimental.noalias.scope.decl(metadata !161046)
  %137 = icmp eq i64 %135, 0
  br i1 %137, label %core::iter::adapters::map::map_try_fold::<purrdf_core::model::RdfQuad, purrdf_core::model::RdfTriple, alloc::vec::in_place_drop::InPlaceDrop<purrdf_core::model::RdfTriple>, core::result::Result<alloc::vec::in_place_drop::InPlaceDrop<purrdf_core::model::RdfTriple>, !>, purrdf_native::py_store::query::materialize_graph::{closure#1}, alloc::vec::in_place_collect::write_in_place_with_drop<purrdf_core::model::RdfTriple>::{closure#0}>::{closure#0} (.exit.i.i.i.i.i), label %bb2.i.i.i4.i.i.i16.i.i.i.i.i.i.i.i.i

bb2.i.i.i4.i.i.i16.i.i.i.i.i.i.i.i.i:             ; preds = %bb2.i15.i.i.i.i.i.i.i.i.i
  %_1.val1.i.i17.i.i.i.i.i.i.i.i.i = load ptr, ptr %121, align 8, !alias.scope !161049, !noalias !161015, !nonnull !3995, !noundef !3995
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %_1.val1.i.i17.i.i.i.i.i.i.i.i.i, i64 noundef %135, i64 noundef range(i64 1, -9223372036854775807) 1) #77, !noalias !161050
  br label %core::iter::adapters::map::map_try_fold::<purrdf_core::model::RdfQuad, purrdf_core::model::RdfTriple, alloc::vec::in_place_drop::InPlaceDrop<purrdf_core::model::RdfTriple>, core::result::Result<alloc::vec::in_place_drop::InPlaceDrop<purrdf_core::model::RdfTriple>, !>, purrdf_native::py_store::query::materialize_graph::{closure#1}, alloc::vec::in_place_collect::write_in_place_with_drop<purrdf_core::model::RdfTriple>::{closure#0}>::{closure#0} (.exit.i.i.i.i.i)

bb4.i.i.i.i.i.i:                                  ; preds = %bb2.i3.i.i.i.i.i.i.i
  %138 = landingpad { ptr, i32 }
          cleanup
  store ptr %_49.i.i.i.i.i, ptr %_15.sroa.4.0._14.sroa_idx.i, align 8, !alias.scope !161051, !noalias !161003
; call core::ptr::drop_glue::<core::option::Option<purrdf_core::diagnostic::RdfLocation>>
  call fastcc void @core::ptr::drop_glue::<core::option::Option<purrdf_core::diagnostic::RdfLocation>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(440) %114) #79, !noalias !161015
; invoke core::ptr::drop_glue::<alloc::vec::in_place_drop::InPlaceDrop<purrdf_core::model::RdfTriple>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::in_place_drop::InPlaceDrop<purrdf_core::model::RdfTriple>>(ptr nonnull %_25.i, ptr nonnull %accum.sroa.4.07.i.i.i.i.i) #79
          to label %bb14.i.i unwind label %terminate.i.i.i.i.i.i, !noalias !161052

terminate.i.i.i.i.i.i:                            ; preds = %bb4.i.i.i.i.i.i
  %139 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #75, !noalias !161052
  unreachable

core::iter::adapters::map::map_try_fold::<purrdf_core::model::RdfQuad, purrdf_core::model::RdfTriple, alloc::vec::in_place_drop::InPlaceDrop<purrdf_core::model::RdfTriple>, core::result::Result<alloc::vec::in_place_drop::InPlaceDrop<purrdf_core::model::RdfTriple>, !>, purrdf_native::py_store::query::materialize_graph::{closure#1}, alloc::vec::in_place_collect::write_in_place_with_drop<purrdf_core::model::RdfTriple>::{closure#0}>::{closure#0} (.exit.i.i.i.i.i): ; preds = %bb2.i.i.i4.i.i.i16.i.i.i.i.i.i.i.i.i, %bb2.i15.i.i.i.i.i.i.i.i.i, %bb5.i.i.i.i.i.i.i.i.i, %bb5.i.i.i.i.i.i.i20
  store i64 2, ptr %accum.sroa.4.07.i.i.i.i.i, align 8, !noalias !161052
  %_5.sroa.4.sroa.4.0._6.sroa.5.8..sroa_idx.i.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %accum.sroa.4.07.i.i.i.i.i, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(192) %_5.sroa.4.sroa.4.0._6.sroa.5.8..sroa_idx.i.i.i.i.i.i, ptr noundef nonnull align 8 dereferenceable(192) %_7.sroa.4.i.i.i.i.i.i, i64 192, i1 false), !noalias !161052
  %_5.sroa.4.sroa.5.0._6.sroa.5.8..sroa_idx.i.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %accum.sroa.4.07.i.i.i.i.i, i64 200
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(80) %_5.sroa.4.sroa.5.0._6.sroa.5.8..sroa_idx.i.i.i.i.i.i, ptr noundef nonnull readonly align 8 dereferenceable(80) %123, i64 80, i1 false), !noalias !161005
  %_5.sroa.4.sroa.6.0._6.sroa.5.8..sroa_idx.i.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %accum.sroa.4.07.i.i.i.i.i, i64 280
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(80) %_5.sroa.4.sroa.6.0._6.sroa.5.8..sroa_idx.i.i.i.i.i.i, ptr noundef nonnull readonly align 8 dereferenceable(80) %122, i64 80, i1 false), !noalias !161005
  %_5.i.i.i.i.i.i.i21 = getelementptr inbounds nuw i8, ptr %accum.sroa.4.07.i.i.i.i.i, i64 360
  call void @llvm.lifetime.end.p0(ptr nonnull %_7.sroa.4.i.i.i.i.i.i)
  call void @llvm.lifetime.end.p0(ptr nonnull %_34.i.i.i.i.i), !noalias !161005
  %_44.not.i.i.i.i.i = icmp eq ptr %_49.i.i.i.i.i, %_29.i
  br i1 %_44.not.i.i.i.i.i, label %bb6.i.i, label %bb11.i.i.i.i.i

bb6.i.i:                                          ; preds = %core::iter::adapters::map::map_try_fold::<purrdf_core::model::RdfQuad, purrdf_core::model::RdfTriple, alloc::vec::in_place_drop::InPlaceDrop<purrdf_core::model::RdfTriple>, core::result::Result<alloc::vec::in_place_drop::InPlaceDrop<purrdf_core::model::RdfTriple>, !>, purrdf_native::py_store::query::materialize_graph::{closure#1}, alloc::vec::in_place_collect::write_in_place_with_drop<purrdf_core::model::RdfTriple>::{closure#0}>::{closure#0} (.exit.i.i.i.i.i), %bb6.i19
  %self.val.i.i.i = phi ptr [ %_25.i, %bb6.i19 ], [ %_29.i, %core::iter::adapters::map::map_try_fold::<purrdf_core::model::RdfQuad, purrdf_core::model::RdfTriple, alloc::vec::in_place_drop::InPlaceDrop<purrdf_core::model::RdfTriple>, core::result::Result<alloc::vec::in_place_drop::InPlaceDrop<purrdf_core::model::RdfTriple>, !>, purrdf_native::py_store::query::materialize_graph::{closure#1}, alloc::vec::in_place_collect::write_in_place_with_drop<purrdf_core::model::RdfTriple>::{closure#0}>::{closure#0} (.exit.i.i.i.i.i) ]
  %accum.sroa.4.0.lcssa.i.i.i.i.i = phi ptr [ %_25.i, %bb6.i19 ], [ %_5.i.i.i.i.i.i.i21, %core::iter::adapters::map::map_try_fold::<purrdf_core::model::RdfQuad, purrdf_core::model::RdfTriple, alloc::vec::in_place_drop::InPlaceDrop<purrdf_core::model::RdfTriple>, core::result::Result<alloc::vec::in_place_drop::InPlaceDrop<purrdf_core::model::RdfTriple>, !>, purrdf_native::py_store::query::materialize_graph::{closure#1}, alloc::vec::in_place_collect::write_in_place_with_drop<purrdf_core::model::RdfTriple>::{closure#0}>::{closure#0} (.exit.i.i.i.i.i) ]
  %140 = ptrtoint ptr %accum.sroa.4.0.lcssa.i.i.i.i.i to i64
  %141 = ptrtoint ptr %_25.i to i64
  %142 = sub nuw i64 %140, %141
  %_0.i.i.i = udiv exact i64 %142, 360
  call void @llvm.lifetime.start.p0(ptr nonnull %dst_guard.i.i), !noalias !161053
  store ptr %_25.i, ptr %dst_guard.i.i, align 8, !noalias !161053
  %143 = getelementptr inbounds nuw i8, ptr %dst_guard.i.i, i64 8
  store i64 %_0.i.i.i, ptr %143, align 8, !noalias !161053
  %144 = getelementptr inbounds nuw i8, ptr %dst_guard.i.i, i64 16
  store i64 %_59.i, ptr %144, align 8, !noalias !161053
  call void @llvm.experimental.noalias.scope.decl(metadata !161054)
  %145 = ptrtoint ptr %_29.i to i64
  %146 = ptrtoint ptr %self.val.i.i.i to i64
  %147 = sub nuw i64 %145, %146
  %148 = udiv exact i64 %147, 440
  store i64 0, ptr %_15.sroa.5.0._14.sroa_idx.i, align 8, !alias.scope !161057, !noalias !161003
  store ptr inttoptr (i64 8 to ptr), ptr %_14.i16, align 8, !alias.scope !161057, !noalias !161003
  store ptr inttoptr (i64 8 to ptr), ptr %_15.sroa.4.0._14.sroa_idx.i, align 8, !alias.scope !161057, !noalias !161003
  store ptr inttoptr (i64 8 to ptr), ptr %_15.sroa.6.0._14.sroa_idx.i, align 8, !alias.scope !161057, !noalias !161003
  %_7.i.i.i.i24457 = icmp eq ptr %_29.i, %self.val.i.i.i
  br i1 %_7.i.i.i.i24457, label %bb7.i.i, label %bb5.i.i.i.i25

bb6.i.i.i.i22:                                    ; preds = %bb5.i.i.i.i25
  %_7.i.i.i.i24 = icmp eq i64 %149, %148
  br i1 %_7.i.i.i.i24, label %bb7.i.i, label %bb5.i.i.i.i25

bb5.i.i.i.i25:                                    ; preds = %bb6.i.i, %bb6.i.i.i.i22
  %_3.sroa.0.0.i.i.i.i23458 = phi i64 [ %149, %bb6.i.i.i.i22 ], [ 0, %bb6.i.i ]
  %_6.i.i.i.i26 = getelementptr inbounds nuw [440 x i8], ptr %self.val.i.i.i, i64 %_3.sroa.0.0.i.i.i.i23458
  %149 = add nuw nsw i64 %_3.sroa.0.0.i.i.i.i23458, 1
; invoke core::ptr::drop_glue::<purrdf_core::model::RdfQuad>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_core::model::RdfQuad>(ptr noalias nofree noundef readonly align 8 dereferenceable(440) %_6.i.i.i.i26)
          to label %bb6.i.i.i.i22 unwind label %cleanup.i.i.i.i27, !noalias !161058

bb4.i.i.i.i28:                                    ; preds = %bb3.i.i.i.i31
  %150 = add i64 %_3.sroa.0.1.i.i.i.i29460, 1
  %_5.i.i.i.i30 = icmp eq i64 %150, %148
  br i1 %_5.i.i.i.i30, label %cleanup3.body.i.i, label %bb3.i.i.i.i31

cleanup.i.i.i.i27:                                ; preds = %bb5.i.i.i.i25
  %151 = landingpad { ptr, i32 }
          cleanup
  %_5.i.i.i.i30459 = icmp eq i64 %149, %148
  br i1 %_5.i.i.i.i30459, label %cleanup3.body.i.i, label %bb3.i.i.i.i31

bb3.i.i.i.i31:                                    ; preds = %cleanup.i.i.i.i27, %bb4.i.i.i.i28
  %_3.sroa.0.1.i.i.i.i29460 = phi i64 [ %150, %bb4.i.i.i.i28 ], [ %149, %cleanup.i.i.i.i27 ]
  %_4.i.i.i.i32 = getelementptr inbounds nuw [440 x i8], ptr %self.val.i.i.i, i64 %_3.sroa.0.1.i.i.i.i29460
; invoke core::ptr::drop_glue::<purrdf_core::model::RdfQuad>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_core::model::RdfQuad>(ptr noalias nofree noundef readonly align 8 dereferenceable(440) %_4.i.i.i.i32) #79
          to label %bb4.i.i.i.i28 unwind label %terminate.i.i.i.i33, !noalias !161058

terminate.i.i.i.i33:                              ; preds = %bb3.i.i.i.i31
  %152 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #75, !noalias !161059
  unreachable

cleanup3.i.i:                                     ; preds = %bb10.i.i
  %153 = landingpad { ptr, i32 }
          cleanup
  br label %cleanup3.body.i.i

cleanup3.body.i.i:                                ; preds = %bb4.i.i.i.i28, %cleanup.i.i.i.i27, %cleanup3.i.i
  %eh.lpad-body9.i.i = phi { ptr, i32 } [ %153, %cleanup3.i.i ], [ %151, %cleanup.i.i.i.i27 ], [ %151, %bb4.i.i.i.i28 ]
; invoke core::ptr::drop_glue::<alloc::vec::in_place_drop::InPlaceDstDataSrcBufDrop<purrdf_core::model::RdfQuad, purrdf_core::model::RdfTriple>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::in_place_drop::InPlaceDstDataSrcBufDrop<purrdf_core::model::RdfQuad, purrdf_core::model::RdfTriple>>(ptr noalias nofree noundef align 8 dereferenceable(24) %dst_guard.i.i) #79
          to label %bb14.i.i unwind label %terminate.i.i, !noalias !161053

bb7.i.i:                                          ; preds = %bb6.i.i.i.i22, %bb6.i.i
  %_5.not.i.i.i = icmp ne i64 %_59.i, 0
  %_7.i.i.i = mul nuw i64 %dst_cap.i.i, 360
  %154 = icmp ne i64 %_12.i.i, %_7.i.i.i
  %or.cond.i.i = select i1 %_5.not.i.i.i, i1 %154, i1 false
  br i1 %or.cond.i.i, label %bb21.i.i, label %bb12.i.i

bb12.i.i:                                         ; preds = %bb28.i.i, %bb9.i.i.i, %bb2.i.i.i, %bb7.i.i
  %dst_buf.sroa.0.0.i.i = phi ptr [ inttoptr (i64 8 to ptr), %bb2.i.i.i ], [ %raw_ptr.i.i.i, %bb28.i.i ], [ %_25.i, %bb7.i.i ], [ inttoptr (i64 8 to ptr), %bb9.i.i.i ]
  call void @llvm.lifetime.end.p0(ptr nonnull %dst_guard.i.i), !noalias !161053
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::model::RdfQuad>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::model::RdfQuad>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(32) %_14.i16)
          to label %.noexc34 unwind label %cleanup9

.noexc34:                                         ; preds = %bb12.i.i
  call void @llvm.lifetime.end.p0(ptr nonnull %_14.i16), !noalias !160985
  call void @llvm.lifetime.start.p0(ptr nonnull %_19.i), !noalias !160985
  store i64 %dst_cap.i.i, ptr %_19.i, align 8, !noalias !160985
  %triples.sroa.5.0._19.sroa_idx.i = getelementptr inbounds nuw i8, ptr %_19.i, i64 8
  store ptr %dst_buf.sroa.0.0.i.i, ptr %triples.sroa.5.0._19.sroa_idx.i, align 8, !noalias !160985
  %triples.sroa.6.0._19.sroa_idx.i = getelementptr inbounds nuw i8, ptr %_19.i, i64 16
  store i64 %_0.i.i.i, ptr %triples.sroa.6.0._19.sroa_idx.i, align 8, !noalias !160985
  %155 = getelementptr inbounds nuw i8, ptr %_19.i, i64 24
  store i64 0, ptr %155, align 8, !noalias !160985
  call void @llvm.lifetime.start.p0(ptr nonnull %_3.i6.i), !noalias !161062
; invoke <pyo3::pyclass_init::PyClassInitializer<purrdf_native::py_store::query::PyQueryTriples>>::create_class_object
  invoke fastcc void @<pyo3::pyclass_init::PyClassInitializer<purrdf_native::py_store::query::PyQueryTriples>>::create_class_object(ptr noalias nofree noundef align 8 captures(address) dereferenceable(56) %_3.i6.i, ptr noalias nofree noundef nonnull readonly align 8 captures(none) dereferenceable(32) %_19.i)
          to label %.noexc35 unwind label %cleanup9

.noexc35:                                         ; preds = %.noexc34
  %_5.i7.i = load i64, ptr %_3.i6.i, align 8, !range !5056, !noalias !161062, !noundef !3995
  %156 = trunc nuw i64 %_5.i7.i to i1
  %157 = getelementptr inbounds nuw i8, ptr %_3.i6.i, i64 8
  %_18.sroa.5.8.copyload.i = load ptr, ptr %157, align 8, !noalias !161066
  br i1 %156, label %bb17.i, label %bb18.i

bb21.i.i:                                         ; preds = %bb7.i.i
  %158 = icmp ult i64 %_12.i.i, 360
  br i1 %158, label %bb2.i.i.i, label %bb28.i.i

bb2.i.i.i:                                        ; preds = %bb21.i.i
  %159 = icmp eq i64 %_12.i.i, 0
  br i1 %159, label %bb12.i.i, label %bb9.i.i.i

bb9.i.i.i:                                        ; preds = %bb2.i.i.i
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %_25.i, i64 noundef %_12.i.i, i64 noundef 8) #77, !noalias !161053
  br label %bb12.i.i

bb28.i.i:                                         ; preds = %bb21.i.i
  %cond.i.i.i = icmp ule i64 %_7.i.i.i, %_12.i.i
  call void @llvm.assume(i1 %cond.i.i.i)
; call __rustc::__rust_realloc
  %raw_ptr.i.i.i = call noundef align 8 ptr @__rustc::__rust_realloc(ptr noundef nonnull %_25.i, i64 noundef %_12.i.i, i64 noundef 8, i64 noundef %_7.i.i.i) #77, !noalias !161053
  %160 = icmp eq ptr %raw_ptr.i.i.i, null
  br i1 %160, label %bb10.i.i, label %bb12.i.i, !prof !4936

bb10.i.i:                                         ; preds = %bb28.i.i
; invoke alloc::alloc::handle_alloc_error
  invoke void @alloc::alloc::handle_alloc_error(i64 noundef 8, i64 noundef %_7.i.i.i) #80
          to label %unreachable.i.i unwind label %cleanup3.i.i, !noalias !161053

unreachable.i.i:                                  ; preds = %bb10.i.i
  unreachable

terminate.i.i:                                    ; preds = %cleanup3.body.i.i, %bb14.i.i
  %161 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #75, !noalias !161053
  unreachable

bb3.i:                                            ; preds = %bb13.i.i
  call void @llvm.lifetime.start.p0(ptr nonnull %_10.i), !noalias !160985
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %_10.i, ptr noundef nonnull align 8 dereferenceable(24) %quads.i, i64 24, i1 false), !noalias !160985
  %162 = getelementptr inbounds nuw i8, ptr %_10.i, i64 24
  store i64 0, ptr %162, align 8, !noalias !160985
  call void @llvm.lifetime.start.p0(ptr nonnull %_3.i.i), !noalias !161067
; invoke <pyo3::pyclass_init::PyClassInitializer<purrdf_native::py_store::query::PyQueryQuads>>::create_class_object
  invoke fastcc void @<pyo3::pyclass_init::PyClassInitializer<purrdf_native::py_store::query::PyQueryQuads>>::create_class_object(ptr noalias nofree noundef align 8 captures(address) dereferenceable(56) %_3.i.i, ptr noalias nofree noundef nonnull readonly align 8 captures(none) dereferenceable(32) %_10.i)
          to label %.noexc36 unwind label %cleanup9

.noexc36:                                         ; preds = %bb3.i
  %_5.i.i = load i64, ptr %_3.i.i, align 8, !range !5056, !noalias !161067, !noundef !3995
  %163 = trunc nuw i64 %_5.i.i to i1
  %164 = getelementptr inbounds nuw i8, ptr %_3.i.i, i64 8
  %_9.sroa.5.8.copyload.i = load ptr, ptr %164, align 8, !noalias !161071
  br i1 %163, label %bb14.i18, label %bb8.i

bb17.i:                                           ; preds = %.noexc35
  %_18.sroa.9.8..sroa_idx.i = getelementptr inbounds nuw i8, ptr %_3.i6.i, i64 16
  %_52.sroa.4.0..sroa_idx.i = getelementptr inbounds nuw i8, ptr %_0, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %_52.sroa.4.0..sroa_idx.i, ptr noundef nonnull align 8 dereferenceable(40) %_18.sroa.9.8..sroa_idx.i, i64 40, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %_3.i6.i), !noalias !161062
  call void @llvm.lifetime.end.p0(ptr nonnull %_19.i), !noalias !160985
  br label %bb6

bb18.i:                                           ; preds = %.noexc35
  call void @llvm.lifetime.end.p0(ptr nonnull %_3.i6.i), !noalias !161062
  call void @llvm.lifetime.end.p0(ptr nonnull %_19.i), !noalias !160985
  br label %bb6

bb14.i18:                                         ; preds = %.noexc36
  %_9.sroa.9.8..sroa_idx.i = getelementptr inbounds nuw i8, ptr %_3.i.i, i64 16
  %_39.sroa.4.0..sroa_idx.i = getelementptr inbounds nuw i8, ptr %_0, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %_39.sroa.4.0..sroa_idx.i, ptr noundef nonnull align 8 dereferenceable(40) %_9.sroa.9.8..sroa_idx.i, i64 40, i1 false)
  br label %bb8.i

bb8.i:                                            ; preds = %.noexc36, %bb14.i18
  %.sink.i = phi i64 [ 1, %bb14.i18 ], [ 0, %.noexc36 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %_3.i.i), !noalias !161067
  call void @llvm.lifetime.end.p0(ptr nonnull %_10.i), !noalias !160985
  br label %bb6

bb2:                                              ; preds = %start
  %165 = getelementptr inbounds nuw i8, ptr %result, i64 8
  %166 = load i8, ptr %165, align 8, !range !4807, !noundef !3995
  call void @llvm.lifetime.start.p0(ptr nonnull %_3.i.i.i), !noalias !161072
  %167 = load atomic i32, ptr getelementptr inbounds nuw (i8, ptr @<purrdf_native::py_store::query::PyQueryBoolean as pyo3::impl_::pyclass::PyClassImpl>::lazy_type_object::TYPE_OBJECT, i64 88) acquire, align 8, !noalias !161077
  %_6.i.i.i.i38 = icmp eq i32 %167, 0
  br i1 %_6.i.i.i.i38, label %<purrdf_native::py_store::query::PyQueryBoolean as pyo3::type_object::PyTypeInfo>::type_object_raw (.exit.i.i), label %<pyo3::impl_::pyclass::lazy_type_object::LazyTypeObject<purrdf_native::py_store::query::PyQueryBoolean>>::get_or_try_init (.exit.i.i.i), !prof !4261

<pyo3::impl_::pyclass::lazy_type_object::LazyTypeObject<purrdf_native::py_store::query::PyQueryBoolean>>::get_or_try_init (.exit.i.i.i): ; preds = %bb2
; invoke <pyo3::impl_::pyclass::lazy_type_object::LazyTypeObject<purrdf_native::py_store::query::PyQueryBoolean>>::try_init
  invoke fastcc void @<pyo3::impl_::pyclass::lazy_type_object::LazyTypeObject<purrdf_native::py_store::query::PyQueryBoolean>>::try_init(ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(56) %_3.i.i.i)
          to label %.noexc45 unwind label %cleanup8

.noexc45:                                         ; preds = %<pyo3::impl_::pyclass::lazy_type_object::LazyTypeObject<purrdf_native::py_store::query::PyQueryBoolean>>::get_or_try_init (.exit.i.i.i)
  %_4.pre.i.i.i = load i64, ptr %_3.i.i.i, align 8, !range !5056, !noalias !161072
  %168 = trunc nuw i64 %_4.pre.i.i.i to i1
  %169 = getelementptr inbounds nuw i8, ptr %_3.i.i.i, i64 8
  br i1 %168, label %bb3.i.i.i, label %<pyo3::impl_::pyclass::lazy_type_object::LazyTypeObject<purrdf_native::py_store::query::PyQueryBoolean>>::get_or_try_init (.exit.i._RNvXs1h_NtNtCsaMxVw72snWA_13purrdf_native8py_store5queryNtB6_14PyQueryBooleanNtNtCsdcnLWgvMhmd_4pyo311type_object10PyTypeInfo15type_object_raw.exit_crit_edge.i.i), !prof !8551

<pyo3::impl_::pyclass::lazy_type_object::LazyTypeObject<purrdf_native::py_store::query::PyQueryBoolean>>::get_or_try_init (.exit.i._RNvXs1h_NtNtCsaMxVw72snWA_13purrdf_native8py_store5queryNtB6_14PyQueryBooleanNtNtCsdcnLWgvMhmd_4pyo311type_object10PyTypeInfo15type_object_raw.exit_crit_edge.i.i): ; preds = %.noexc45
  %_2.i.pre.i.i = load ptr, ptr %169, align 8, !noalias !161072
  br label %<purrdf_native::py_store::query::PyQueryBoolean as pyo3::type_object::PyTypeInfo>::type_object_raw (.exit.i.i)

bb3.i.i.i:                                        ; preds = %.noexc45
; invoke pyo3::impl_::pyclass::lazy_type_object::type_object_init_failed
  invoke void @pyo3::impl_::pyclass::lazy_type_object::type_object_init_failed(ptr noalias nofree noundef nonnull readonly align 8 captures(none) dereferenceable(48) %169, ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @alloc_c061a48059fbd8e0b91c7f3c14785c78, i64 noundef 12) #80
          to label %.noexc46 unwind label %cleanup8

.noexc46:                                         ; preds = %bb3.i.i.i
  unreachable

<purrdf_native::py_store::query::PyQueryBoolean as pyo3::type_object::PyTypeInfo>::type_object_raw (.exit.i.i): ; preds = %<pyo3::impl_::pyclass::lazy_type_object::LazyTypeObject<purrdf_native::py_store::query::PyQueryBoolean>>::get_or_try_init (.exit.i._RNvXs1h_NtNtCsaMxVw72snWA_13purrdf_native8py_store5queryNtB6_14PyQueryBooleanNtNtCsdcnLWgvMhmd_4pyo311type_object10PyTypeInfo15type_object_raw.exit_crit_edge.i.i), %bb2
  %_2.i.i.i39 = phi ptr [ %_2.i.pre.i.i, %<pyo3::impl_::pyclass::lazy_type_object::LazyTypeObject<purrdf_native::py_store::query::PyQueryBoolean>>::get_or_try_init (.exit.i._RNvXs1h_NtNtCsaMxVw72snWA_13purrdf_native8py_store5queryNtB6_14PyQueryBooleanNtNtCsdcnLWgvMhmd_4pyo311type_object10PyTypeInfo15type_object_raw.exit_crit_edge.i.i) ], [ getelementptr inbounds nuw (i8, ptr @<purrdf_native::py_store::query::PyQueryBoolean as pyo3::impl_::pyclass::PyClassImpl>::lazy_type_object::TYPE_OBJECT, i64 80), %bb2 ]
  %_7.i.i.i40 = load ptr, ptr %_2.i.i.i39, align 8, !noalias !161072, !nonnull !3995, !noundef !3995
  call void @llvm.lifetime.end.p0(ptr nonnull %_3.i.i.i), !noalias !161072
  call void @llvm.lifetime.start.p0(ptr nonnull %_5.i.i.i), !noalias !161080
; invoke <pyo3::internal::pyclass_init::PyNativeTypeInitializer<_> as pyo3::internal::pyclass_init::PyObjectInit<_>>::into_new_object::inner
  invoke void @<pyo3::internal::pyclass_init::PyNativeTypeInitializer<_> as pyo3::internal::pyclass_init::PyObjectInit<_>>::into_new_object::inner(ptr noalias nofree noundef nonnull sret([56 x i8]) align 8 captures(none) dereferenceable(56) %_5.i.i.i, ptr noundef nonnull @PyBaseObject_Type, ptr noundef nonnull %_7.i.i.i40)
          to label %.noexc47 unwind label %cleanup8

.noexc47:                                         ; preds = %<purrdf_native::py_store::query::PyQueryBoolean as pyo3::type_object::PyTypeInfo>::type_object_raw (.exit.i.i)
  %_14.i.i.i = load i64, ptr %_5.i.i.i, align 8, !range !5056, !noalias !161080, !noundef !3995
  %170 = trunc nuw i64 %_14.i.i.i to i1
  %171 = getelementptr inbounds nuw i8, ptr %_5.i.i.i, i64 8
  %_16.sroa.0.0.copyload.i.i.i = load ptr, ptr %171, align 8, !noalias !161080
  br i1 %170, label %bb29, label %bb30

bb24:                                             ; preds = %bb4.i.i, %bb2.i.i.i6.i.i
  call void @llvm.lifetime.end.p0(ptr nonnull %residual.i), !noalias !160772
  %_46.sroa.6.0..sroa_idx = getelementptr inbounds nuw i8, ptr %_0, i64 32
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %_46.sroa.6.0..sroa_idx, ptr noundef nonnull align 8 dereferenceable(24) %_7.sroa.12, i64 24, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %_7.sroa.12)
  %172 = getelementptr inbounds nuw i8, ptr %_0, i64 8
  store i64 %_7.sroa.6.8.copyload82, ptr %172, align 8
  %_46.sroa.4.0..sroa_idx = getelementptr inbounds nuw i8, ptr %_0, i64 16
  store ptr %_7.sroa.10.8.copyload84, ptr %_46.sroa.4.0..sroa_idx, align 8
  %_46.sroa.5.0..sroa_idx = getelementptr inbounds nuw i8, ptr %_0, i64 24
  store i64 %_7.sroa.11.8.copyload86, ptr %_46.sroa.5.0..sroa_idx, align 8
  store i64 1, ptr %_0, align 8
  call void @llvm.lifetime.end.p0(ptr nonnull %rows)
  call void @llvm.experimental.noalias.scope.decl(metadata !161083)
  %173 = getelementptr inbounds nuw i8, ptr %variables, i64 8
  %_1.val.i = load ptr, ptr %173, align 8, !alias.scope !161083, !nonnull !3995, !noundef !3995
  %174 = getelementptr inbounds nuw i8, ptr %variables, i64 16
  %_1.val1.i = load i64, ptr %174, align 8, !alias.scope !161083, !noundef !3995
  call void @llvm.experimental.noalias.scope.decl(metadata !161086)
  %_710.i.i.i = icmp eq i64 %_1.val1.i, 0
  br i1 %_710.i.i.i, label %bb4.i, label %bb5.i.i.i

bb5.i.i.i:                                        ; preds = %bb24, %core::ptr::drop_glue::<alloc::string::String> (.exit.i.i.i)
  %_3.sroa.0.011.i.i.i = phi i64 [ %175, %core::ptr::drop_glue::<alloc::string::String> (.exit.i.i.i) ], [ 0, %bb24 ]
  %_6.i.i.i = getelementptr inbounds nuw [24 x i8], ptr %_1.val.i, i64 %_3.sroa.0.011.i.i.i
  %175 = add nuw nsw i64 %_3.sroa.0.011.i.i.i, 1
  call void @llvm.experimental.noalias.scope.decl(metadata !161089)
  %_1.val.i.i.i.i = load i64, ptr %_6.i.i.i, align 8, !alias.scope !161092, !noalias !161083
  %176 = icmp eq i64 %_1.val.i.i.i.i, 0
  br i1 %176, label %core::ptr::drop_glue::<alloc::string::String> (.exit.i.i.i), label %bb2.i.i.i4.i.i.i.i.i

bb2.i.i.i4.i.i.i.i.i:                             ; preds = %bb5.i.i.i
  %177 = getelementptr inbounds nuw i8, ptr %_6.i.i.i, i64 8
  %_1.val1.i.i.i.i = load ptr, ptr %177, align 8, !alias.scope !161092, !noalias !161083, !nonnull !3995, !noundef !3995
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %_1.val1.i.i.i.i, i64 noundef %_1.val.i.i.i.i, i64 noundef range(i64 1, -9223372036854775807) 1) #77, !noalias !161093
  br label %core::ptr::drop_glue::<alloc::string::String> (.exit.i.i.i)

core::ptr::drop_glue::<alloc::string::String> (.exit.i.i.i): ; preds = %bb2.i.i.i4.i.i.i.i.i, %bb5.i.i.i
  %_7.i.i.i48 = icmp eq i64 %175, %_1.val1.i
  br i1 %_7.i.i.i48, label %bb4.i, label %bb5.i.i.i

bb4.i:                                            ; preds = %core::ptr::drop_glue::<alloc::string::String> (.exit.i.i.i), %bb24
  %_1.val4.i = load i64, ptr %variables, align 8, !alias.scope !161083
  %178 = icmp eq i64 %_1.val4.i, 0
  br i1 %178, label %bb9, label %bb2.i.i.i6.i

bb2.i.i.i6.i:                                     ; preds = %bb4.i
  %alloc_size.i.i.i.i7.i = mul nuw i64 %_1.val4.i, 24
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %_1.val.i, i64 noundef %alloc_size.i.i.i.i7.i, i64 noundef range(i64 1, -9223372036854775807) 8) #77, !noalias !161083
  br label %bb9

bb25:                                             ; preds = %bb1.i
  call void @llvm.lifetime.end.p0(ptr nonnull %residual.i), !noalias !160772
  call void @llvm.lifetime.end.p0(ptr nonnull %_7.sroa.12)
  store i64 %rows4, ptr %rows, align 8
  %val.sroa.4.0.rows.sroa_idx = getelementptr inbounds nuw i8, ptr %rows, i64 8
  store ptr %rows3, ptr %val.sroa.4.0.rows.sroa_idx, align 8
  %val.sroa.5.0.rows.sroa_idx = getelementptr inbounds nuw i8, ptr %rows, i64 16
  store i64 %_0.i.i.i.i.i.i, ptr %val.sroa.5.0.rows.sroa_idx, align 8
  call void @llvm.lifetime.start.p0(ptr nonnull %_15)
  %_17.sroa.0.0.copyload = load i64, ptr %variables, align 8
  %_17.sroa.4.0.variables.sroa_idx = getelementptr inbounds nuw i8, ptr %variables, i64 8
  %_17.sroa.4.0.copyload = load ptr, ptr %_17.sroa.4.0.variables.sroa_idx, align 8, !nonnull !3995, !noundef !3995
  %_17.sroa.5.0.variables.sroa_idx = getelementptr inbounds nuw i8, ptr %variables, i64 16
  %_17.sroa.5.0.copyload = load i64, ptr %_17.sroa.5.0.variables.sroa_idx, align 8
  %_26.i = icmp ult i64 %_17.sroa.5.0.copyload, 384307168202282326
  call void @llvm.assume(i1 %_26.i)
  %array_size.i.i = mul nuw nsw i64 %_17.sroa.5.0.copyload, 24
; invoke alloc::sync::arcinner_layout_for_value_layout
  %179 = invoke { i64, i64 } @alloc::sync::arcinner_layout_for_value_layout(i64 noundef range(i64 1, 9) 8, i64 noundef %array_size.i.i)
          to label %.noexc53 unwind label %bb17

.noexc53:                                         ; preds = %bb25
  %layout.0.i.i = extractvalue { i64, i64 } %179, 0
  %layout.1.i.i = extractvalue { i64, i64 } %179, 1
  %180 = icmp eq i64 %layout.1.i.i, 0
  br i1 %180, label %bb2.i.i.i.i.i52, label %bb1.i.i.i.i.i

bb2.i.i.i.i.i52:                                  ; preds = %.noexc53
  %self3.i.i.i.i.i = inttoptr i64 %layout.0.i.i to ptr
  br label %<alloc::sync::Arc<[alloc::string::String]>>::allocate_for_slice_in::{closure#0} (.exit.i.i)

bb1.i.i.i.i.i:                                    ; preds = %.noexc53
; call __rustc::__rust_no_alloc_shim_is_unstable_v2
  call void @__rustc::__rust_no_alloc_shim_is_unstable_v2() #77, !noalias !161094
; call __rustc::__rust_alloc
  %181 = call noundef ptr @__rustc::__rust_alloc(i64 noundef %layout.1.i.i, i64 noundef range(i64 1, -9223372036854775807) %layout.0.i.i) #77, !noalias !161094
  br label %<alloc::sync::Arc<[alloc::string::String]>>::allocate_for_slice_in::{closure#0} (.exit.i.i)

<alloc::sync::Arc<[alloc::string::String]>>::allocate_for_slice_in::{closure#0} (.exit.i.i): ; preds = %bb1.i.i.i.i.i, %bb2.i.i.i.i.i52
  %_0.sroa.0.0.i.i.i.i.i = phi ptr [ %self3.i.i.i.i.i, %bb2.i.i.i.i.i52 ], [ %181, %bb1.i.i.i.i.i ]
  %182 = icmp eq ptr %_0.sroa.0.0.i.i.i.i.i, null
  br i1 %182, label %bb9.i.i, label %bb4.i.i49, !prof !4025

bb9.i.i:                                          ; preds = %<alloc::sync::Arc<[alloc::string::String]>>::allocate_for_slice_in::{closure#0} (.exit.i.i)
; invoke alloc::alloc::handle_alloc_error
  invoke void @alloc::alloc::handle_alloc_error(i64 noundef %layout.0.i.i, i64 noundef %layout.1.i.i) #80
          to label %.noexc54 unwind label %bb17

.noexc54:                                         ; preds = %bb9.i.i
  unreachable

bb4.i.i49:                                        ; preds = %<alloc::sync::Arc<[alloc::string::String]>>::allocate_for_slice_in::{closure#0} (.exit.i.i)
  store i64 1, ptr %_0.sroa.0.0.i.i.i.i.i, align 8, !noalias !161094
  %183 = getelementptr inbounds nuw i8, ptr %_0.sroa.0.0.i.i.i.i.i, i64 8
  store i64 1, ptr %183, align 8, !noalias !161094
  %_8.0.i = getelementptr inbounds nuw i8, ptr %_0.sroa.0.0.i.i.i.i.i, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr nonnull align 8 %_8.0.i, ptr nonnull align 8 %_17.sroa.4.0.copyload, i64 %array_size.i.i, i1 false), !noalias !161094
  %184 = icmp eq i64 %_17.sroa.0.0.copyload, 0
  br i1 %184, label %bb26, label %bb2.i.i.i6.i.i50

bb2.i.i.i6.i.i50:                                 ; preds = %bb4.i.i49
  %alloc_size.i.i.i.i7.i.i51 = mul nuw i64 %_17.sroa.0.0.copyload, 24
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %_17.sroa.4.0.copyload, i64 noundef %alloc_size.i.i.i.i7.i.i51, i64 noundef range(i64 1, -9223372036854775807) 8) #77, !noalias !161097
  br label %bb26

cleanup6:                                         ; preds = %bb26
  %lpad.thr_comm.split-lp = landingpad { ptr, i32 }
          cleanup
  br label %bb22

bb26:                                             ; preds = %bb2.i.i.i6.i.i50, %bb4.i.i49
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %_15, ptr noundef nonnull align 8 dereferenceable(24) %rows, i64 24, i1 false)
  %185 = getelementptr inbounds nuw i8, ptr %_15, i64 24
  store ptr %_0.sroa.0.0.i.i.i.i.i, ptr %185, align 8
  %186 = getelementptr inbounds nuw i8, ptr %_15, i64 32
  store i64 %_17.sroa.5.0.copyload, ptr %186, align 8
  %187 = getelementptr inbounds nuw i8, ptr %_15, i64 40
  store i64 0, ptr %187, align 8
  call void @llvm.lifetime.start.p0(ptr nonnull %_3.i), !noalias !161100
; invoke <pyo3::pyclass_init::PyClassInitializer<purrdf_native::py_store::query::PyQuerySolutions>>::create_class_object
  invoke fastcc void @<pyo3::pyclass_init::PyClassInitializer<purrdf_native::py_store::query::PyQuerySolutions>>::create_class_object(ptr noalias nofree noundef align 8 captures(address) dereferenceable(56) %_3.i, ptr noalias nofree noundef nonnull readonly align 8 captures(none) dereferenceable(48) %_15)
          to label %.noexc59 unwind label %cleanup6

.noexc59:                                         ; preds = %bb26
  %_5.i = load i64, ptr %_3.i, align 8, !range !5056, !noalias !161100, !noundef !3995
  %188 = trunc nuw i64 %_5.i to i1
  %189 = getelementptr inbounds nuw i8, ptr %_3.i, i64 8
  %_14.sroa.5.8.copyload = load ptr, ptr %189, align 8, !noalias !161104
  br i1 %188, label %bb27, label %bb28

bb27:                                             ; preds = %.noexc59
  %_14.sroa.9.8..sroa_idx = getelementptr inbounds nuw i8, ptr %_3.i, i64 16
  %_53.sroa.4.0..sroa_idx = getelementptr inbounds nuw i8, ptr %_0, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %_53.sroa.4.0..sroa_idx, ptr noundef nonnull align 8 dereferenceable(40) %_14.sroa.9.8..sroa_idx, i64 40, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %_3.i), !noalias !161100
  call void @llvm.lifetime.end.p0(ptr nonnull %_15)
  %190 = getelementptr inbounds nuw i8, ptr %_0, i64 8
  store ptr %_14.sroa.5.8.copyload, ptr %190, align 8
  store i64 1, ptr %_0, align 8
  call void @llvm.lifetime.end.p0(ptr nonnull %rows)
  br label %bb9

bb28:                                             ; preds = %.noexc59
  call void @llvm.lifetime.end.p0(ptr nonnull %_3.i), !noalias !161100
  call void @llvm.lifetime.end.p0(ptr nonnull %_15)
  %191 = getelementptr inbounds nuw i8, ptr %_0, i64 8
  store ptr %_14.sroa.5.8.copyload, ptr %191, align 8
  store i64 0, ptr %_0, align 8
  call void @llvm.lifetime.end.p0(ptr nonnull %rows)
  call void @llvm.lifetime.end.p0(ptr nonnull %variables)
  br label %bb14

bb14:                                             ; preds = %bb30, %bb7, %bb28
  %192 = load i64, ptr %result, align 8, !range !20940, !noundef !3995
  %193 = icmp sgt i64 %192, -1
  br i1 %193, label %bb13, label %bb10

bb9:                                              ; preds = %bb2.i.i.i6.i, %bb4.i, %bb27
  call void @llvm.lifetime.end.p0(ptr nonnull %variables)
  br label %bb16

bb17:                                             ; preds = %bb9.i.i, %bb25
  %lpad.thr_comm = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<alloc::vec::Vec<alloc::vec::Vec<core::option::Option<purrdf_core::model::RdfTerm>>>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<alloc::vec::Vec<core::option::Option<purrdf_core::model::RdfTerm>>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %rows) #79
          to label %bb22 unwind label %terminate

terminate:                                        ; preds = %bb2.i.i76, %bb2.i.i, %bb17
  %194 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #75
  unreachable

bb22:                                             ; preds = %bb19, %cleanup6, %bb17, %cleanup9.body, %bb2.i.i, %cleanup8
  %.pn14 = phi { ptr, i32 } [ %197, %cleanup8 ], [ %eh.lpad-body, %bb19 ], [ %eh.lpad-body37, %cleanup9.body ], [ %eh.lpad-body37, %bb2.i.i ], [ %lpad.thr_comm.split-lp, %cleanup6 ], [ %lpad.thr_comm, %bb17 ]
  %195 = load i64, ptr %result, align 8, !range !20940, !noundef !3995
  %196 = icmp sgt i64 %195, -1
  br i1 %196, label %bb21, label %bb12

cleanup8:                                         ; preds = %bb2.i.i63, %<purrdf_native::py_store::query::PyQueryBoolean as pyo3::type_object::PyTypeInfo>::type_object_raw (.exit.i.i), %bb3.i.i.i, %<pyo3::impl_::pyclass::lazy_type_object::LazyTypeObject<purrdf_native::py_store::query::PyQueryBoolean>>::get_or_try_init (.exit.i.i.i)
  %197 = landingpad { ptr, i32 }
          cleanup
  br label %bb22

bb16:                                             ; preds = %bb29, %bb9
  %198 = load i64, ptr %result, align 8, !range !20940, !noundef !3995
  %199 = icmp sgt i64 %198, -1
  br i1 %199, label %bb15, label %bb10

bb19:                                             ; preds = %bb15.i, %cleanup.body.i.i, %bb2.i.i.i.i.i, %bb14.i
  %eh.lpad-body = phi { ptr, i32 } [ %102, %cleanup.body.i.i ], [ %eh.lpad-body.i, %bb14.i ], [ %eh.lpad-body.i, %bb15.i ], [ %102, %bb2.i.i.i.i.i ]
; call core::ptr::drop_glue::<alloc::vec::Vec<alloc::string::String>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<alloc::string::String>>(ptr noalias nofree noundef align 8 dereferenceable(24) %variables) #79
  br label %bb22

cleanup9:                                         ; preds = %bb3.i, %.noexc34, %bb12.i.i, %bb3
  %200 = landingpad { ptr, i32 }
          cleanup
  br label %cleanup9.body

cleanup9.body:                                    ; preds = %bb14.i.i, %cleanup9
  %eh.lpad-body37 = phi { ptr, i32 } [ %200, %cleanup9 ], [ %.pn.i.i, %bb14.i.i ]
  call void @llvm.experimental.noalias.scope.decl(metadata !161105)
  call void @llvm.experimental.noalias.scope.decl(metadata !161108)
  %_10.i.i = load ptr, ptr %graph, align 8, !alias.scope !161111, !nonnull !3995, !noundef !3995
  %_2.i.i = atomicrmw sub ptr %_10.i.i, i64 1 release, align 8, !noalias !161111
  %201 = icmp eq i64 %_2.i.i, 1
  br i1 %201, label %bb2.i.i, label %bb22

bb2.i.i:                                          ; preds = %cleanup9.body
  fence acquire
; invoke <alloc::sync::Arc<purrdf_core::ir::dataset::RdfDataset>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_core::ir::dataset::RdfDataset>>::drop_slow(ptr noalias nofree noundef nonnull align 8 dereferenceable(8) %graph) #81
          to label %bb22 unwind label %terminate

bb6:                                              ; preds = %bb8.i, %bb18.i, %bb17.i
  %_9.sroa.5.8.copyload.i.sink = phi ptr [ %_9.sroa.5.8.copyload.i, %bb8.i ], [ %_18.sroa.5.8.copyload.i, %bb18.i ], [ %_18.sroa.5.8.copyload.i, %bb17.i ]
  %.sink.i.sink = phi i64 [ %.sink.i, %bb8.i ], [ 0, %bb18.i ], [ 1, %bb17.i ]
  %202 = getelementptr inbounds nuw i8, ptr %_0, i64 8
  store ptr %_9.sroa.5.8.copyload.i.sink, ptr %202, align 8, !alias.scope !160985
  store i64 %.sink.i.sink, ptr %_0, align 8, !alias.scope !160985
  call void @llvm.lifetime.end.p0(ptr nonnull %quads.i), !noalias !160985
  call void @llvm.experimental.noalias.scope.decl(metadata !161112)
  call void @llvm.experimental.noalias.scope.decl(metadata !161115)
  %_10.i.i61 = load ptr, ptr %graph, align 8, !alias.scope !161118, !nonnull !3995, !noundef !3995
  %_2.i.i62 = atomicrmw sub ptr %_10.i.i61, i64 1 release, align 8, !noalias !161118
  %203 = icmp eq i64 %_2.i.i62, 1
  br i1 %203, label %bb2.i.i63, label %bb7

bb2.i.i63:                                        ; preds = %bb6
  fence acquire
; invoke <alloc::sync::Arc<purrdf_core::ir::dataset::RdfDataset>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_core::ir::dataset::RdfDataset>>::drop_slow(ptr noalias nofree noundef nonnull align 8 dereferenceable(8) %graph) #81
          to label %bb7 unwind label %cleanup8

bb7:                                              ; preds = %bb6, %bb2.i.i63
  call void @llvm.lifetime.end.p0(ptr nonnull %graph)
  br label %bb14

bb29:                                             ; preds = %.noexc47
  %_16.sroa.4.0..sroa_idx.i.i.i = getelementptr inbounds nuw i8, ptr %_5.i.i.i, i64 16
  %_60.sroa.4.0..sroa_idx = getelementptr inbounds nuw i8, ptr %_0, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %_60.sroa.4.0..sroa_idx, ptr noundef nonnull align 8 dereferenceable(40) %_16.sroa.4.0..sroa_idx.i.i.i, i64 40, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %_5.i.i.i), !noalias !161080
  %204 = getelementptr inbounds nuw i8, ptr %_0, i64 8
  store ptr %_16.sroa.0.0.copyload.i.i.i, ptr %204, align 8
  store i64 1, ptr %_0, align 8
  br label %bb16

bb30:                                             ; preds = %.noexc47
  call void @llvm.lifetime.end.p0(ptr nonnull %_5.i.i.i), !noalias !161080
  %_0.i.i.i.i = getelementptr inbounds nuw i8, ptr %_16.sroa.0.0.copyload.i.i.i, i64 16
  store i8 %166, ptr %_0.i.i.i.i, align 8, !noalias !161080
  %_21.sroa.5.0._0.i.sroa_idx.i.i.i = getelementptr inbounds nuw i8, ptr %_16.sroa.0.0.copyload.i.i.i, i64 24
  store i64 0, ptr %_21.sroa.5.0._0.i.sroa_idx.i.i.i, align 8, !noalias !161080
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %_16.sroa.0.0.copyload.i.i.i) ]
  %205 = getelementptr inbounds nuw i8, ptr %_0, i64 8
  store ptr %_16.sroa.0.0.copyload.i.i.i, ptr %205, align 8
  store i64 0, ptr %_0, align 8
  br label %bb14

bb13:                                             ; preds = %bb14
  %206 = getelementptr inbounds nuw i8, ptr %result, i64 48
  call void @llvm.experimental.noalias.scope.decl(metadata !161119)
  call void @llvm.experimental.noalias.scope.decl(metadata !161122)
  %_10.i.i66 = load ptr, ptr %206, align 8, !alias.scope !161125, !nonnull !3995, !noundef !3995
  %_2.i.i67 = atomicrmw sub ptr %_10.i.i66, i64 1 release, align 8, !noalias !161125
  %207 = icmp eq i64 %_2.i.i67, 1
  br i1 %207, label %bb10.sink.split, label %bb10

bb10.sink.split:                                  ; preds = %bb13, %bb15
  %.sink = phi ptr [ %208, %bb15 ], [ %206, %bb13 ]
  fence acquire
; call <alloc::sync::Arc<purrdf_core::ir::dataset::RdfDataset>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_core::ir::dataset::RdfDataset>>::drop_slow(ptr noalias nofree noundef nonnull align 8 dereferenceable(8) %.sink) #81
  br label %bb10

bb10:                                             ; preds = %bb10.sink.split, %bb15, %bb13, %bb16, %bb14
  ret void

bb15:                                             ; preds = %bb16
  %208 = getelementptr inbounds nuw i8, ptr %result, i64 48
  call void @llvm.experimental.noalias.scope.decl(metadata !161126)
  call void @llvm.experimental.noalias.scope.decl(metadata !161129)
  %_10.i.i70 = load ptr, ptr %208, align 8, !alias.scope !161132, !nonnull !3995, !noundef !3995
  %_2.i.i71 = atomicrmw sub ptr %_10.i.i70, i64 1 release, align 8, !noalias !161132
  %209 = icmp eq i64 %_2.i.i71, 1
  br i1 %209, label %bb10.sink.split, label %bb10

bb21:                                             ; preds = %bb22
  %210 = getelementptr inbounds nuw i8, ptr %result, i64 48
  call void @llvm.experimental.noalias.scope.decl(metadata !161133)
  call void @llvm.experimental.noalias.scope.decl(metadata !161136)
  %_10.i.i74 = load ptr, ptr %210, align 8, !alias.scope !161139, !nonnull !3995, !noundef !3995
  %_2.i.i75 = atomicrmw sub ptr %_10.i.i74, i64 1 release, align 8, !noalias !161139
  %211 = icmp eq i64 %_2.i.i75, 1
  br i1 %211, label %bb2.i.i76, label %bb12

bb2.i.i76:                                        ; preds = %bb21
  fence acquire
; invoke <alloc::sync::Arc<purrdf_core::ir::dataset::RdfDataset>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_core::ir::dataset::RdfDataset>>::drop_slow(ptr noalias nofree noundef nonnull align 8 dereferenceable(8) %210) #81
          to label %bb12 unwind label %terminate

bb12:                                             ; preds = %bb21, %bb2.i.i76, %bb22
  resume { ptr, i32 } %.pn14
}

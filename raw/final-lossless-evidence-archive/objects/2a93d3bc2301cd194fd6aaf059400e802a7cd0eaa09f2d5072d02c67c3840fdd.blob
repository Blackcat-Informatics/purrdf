define internal fastcc void @purrdf_sparql_eval::row_checkpoint::commit_items::<purrdf_core::ir::dataset::RdfDataset, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, <purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::commit_rows<purrdf_core::ir::dataset::RdfDataset, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>::{closure#0}>::{closure#5}(ptr nofree nonnull captures(none) %0, ptr nofree nonnull captures(none) %1, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %2, i64 noundef %3) unnamed_addr #2 !guid !65278 {
  %5 = alloca [80 x i8], align 8
  %6 = load i64, ptr %0, align 8, !noundef !1740
  %7 = icmp ult i64 %6, %3
  br i1 %7, label %8, label %.loopexit

8:                                                ; preds = %4
  %9 = getelementptr inbounds nuw i8, ptr %1, i64 24
  %10 = getelementptr inbounds nuw i8, ptr %1, i64 8
  %11 = getelementptr inbounds nuw i8, ptr %5, i64 8
  %12 = getelementptr inbounds nuw i8, ptr %2, i64 888
  br label %13

13:                                               ; preds = %22, %8
  call void @llvm.experimental.noalias.scope.decl(metadata !65279)
  %14 = load ptr, ptr %9, align 8, !alias.scope !65279, !noalias !65282, !nonnull !1740, !noundef !1740
  %15 = load ptr, ptr %10, align 8, !alias.scope !65279, !noalias !65282, !nonnull !1740, !noundef !1740
  %16 = icmp eq ptr %15, %14
  br i1 %16, label %.loopexit, label %17

17:                                               ; preds = %13
  %18 = getelementptr inbounds nuw i8, ptr %15, i64 88
  store ptr %18, ptr %10, align 8, !alias.scope !65279, !noalias !65282
  %19 = getelementptr inbounds nuw i8, ptr %15, i64 8
  %20 = load i64, ptr %19, align 8, !noalias !65279
  %21 = icmp eq i64 %20, -1
  br i1 %21, label %.loopexit, label %22

.loopexit:                                        ; preds = %13, %17, %22, %4
  ret void

22:                                               ; preds = %17
  %23 = getelementptr inbounds nuw i8, ptr %15, i64 16
  %24 = load i64, ptr %15, align 8, !noalias !65279
  call void @llvm.lifetime.start.p0(ptr nonnull %5)
  store i64 %20, ptr %5, align 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %11, ptr noundef nonnull align 8 dereferenceable(72) %23, i64 72, i1 false)
  %25 = load i64, ptr %0, align 8, !noundef !1740
  %26 = add i64 %25, 1
  store i64 %26, ptr %0, align 8
; call <purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint
  call void @<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint(ptr noalias nofree noundef nonnull align 8 dereferenceable(184) %12, i64 noundef %24, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(80) %5)
  call void @llvm.lifetime.end.p0(ptr nonnull %5)
  %27 = load i64, ptr %0, align 8, !noundef !1740
  %28 = icmp ult i64 %27, %3
  br i1 %28, label %13, label %.loopexit
}

define internal fastcc void @<purrdf_native::py_store::quad_store::PyQuadStore>::query(ptr dead_on_unwind noalias nofree noundef nonnull writable align 8 captures(none) dereferenceable(56) %_0, ptr noundef nonnull align 8 %self, ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) %query.0, i64 noundef %query.1, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable_or_null(8) %substitutions, ptr noalias nofree noundef nonnull readonly align 8 captures(none) dead_on_return dereferenceable(24) %extension_namespaces, ptr noalias nofree noundef nonnull readonly align 8 captures(none) dead_on_return dereferenceable(24) %property_fn_namespaces, ptr noalias nofree noundef nonnull readonly align 8 captures(none) dead_on_return dereferenceable(48) %standpoint_predicates, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable_or_null(8) %relations, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable_or_null(8) %relations_from_graph, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable_or_null(8) %path_relations, ptr noalias nofree noundef nonnull readonly align 8 captures(none) dead_on_return dereferenceable(24) %aggregate_namespace, ptr noalias nofree noundef readonly captures(address, read_provenance) %xpath_regex.0, i64 %xpath_regex.1, ptr noalias nofree noundef nonnull readonly align 8 captures(none) dead_on_return dereferenceable(24) %division) unnamed_addr #1 personality ptr @rust_eh_personality !guid !107640 {
start:
  %_4.i.i.i.i.i.i.i = alloca [56 x i8], align 8
  %_9.i.i.i.i = alloca [56 x i8], align 8
  %_3.i1.i = alloca [48 x i8], align 8
  %guard.i.i = alloca [4 x i8], align 4
  %args.i49.i.i.i.i = alloca [16 x i8], align 8
  %_3.i50.i.i.i.i = alloca [24 x i8], align 8
  %_20.i.i.i.i.i.i.i.i = alloca [176 x i8], align 8
  %_5.i.i.i.i.i.i.i.i = alloca [96 x i8], align 8
  %prepared.i.i.i.i.i.i.i.i = alloca [8 x i8], align 8
  %admitted.i.i.i.i.i.i.i.i = alloca [288 x i8], align 8
  %result.i.i.i.i.i.i = alloca [96 x i8], align 8
  %_12.i.i.i.i.i = alloca [24 x i8], align 8
  %_2.i.i.i.i.i = alloca [24 x i8], align 8
  %args.i.i.i.i.i = alloca [16 x i8], align 8
  %_3.i.i.i.i.i = alloca [24 x i8], align 8
  %_95.i.i.i.i = alloca [96 x i8], align 8
  %_94.i.i.i.i = alloca [48 x i8], align 8
  %_47.i.i.i.i = alloca [96 x i8], align 8
  %parser_options.i.i.i.i = alloca [72 x i8], align 8
  %_31.i.i.i.i = alloca [352 x i8], align 8
  %_30.sroa.6.i.i.sroa.0.i.i = alloca [16 x i8], align 8
  %_29.i.i.i.i = alloca [352 x i8], align 8
  %_21.sroa.6.i.i.sroa.6.i.i = alloca [16 x i8], align 8
  %_21.sroa.7.i.i.i.i = alloca [32 x i8], align 8
  %engine.i.i.i.i = alloca [520 x i8], align 8
  %aggregates.i.i.i.i = alloca [40 x i8], align 8
  %_10.i.i.i.i = alloca [80 x i8], align 8
  %_9.sroa.5.i.i.sroa.0.i.i = alloca [16 x i8], align 8
  %registry.i.i.i.i = alloca [72 x i8], align 8
  %_5.i.i.i.i = alloca [96 x i8], align 8
  %dataset.i.i.i.i = alloca [8 x i8], align 8
  %_guard.i.i.i = alloca [16 x i8], align 8
  %_26.i.i = alloca [72 x i8], align 8
  %result.i.i = alloca [56 x i8], align 8
  %_33.i.i = alloca [272 x i8], align 8
  %_32.sroa.10.i.i = alloca [16 x i8], align 8
  %_31.sroa.6.sroa.0.i.i = alloca [16 x i8], align 8
  %config.sroa.0.i.i = alloca [96 x i8], align 8
  %_15.i.i = alloca [56 x i8], align 8
  %_14.sroa.5.i.i = alloca [48 x i8], align 8
  %specs.i.i = alloca [24 x i8], align 8
  %_9.i.i = alloca [56 x i8], align 8
  %_8.sroa.5.i.i = alloca [48 x i8], align 8
  %subs.i.i = alloca [24 x i8], align 8
  %_3.i.i = alloca [56 x i8], align 8
  %_4.sroa.0.i = alloca [16 x i8], align 8
  %_14 = alloca [216 x i8], align 8
  call void @llvm.lifetime.start.p0(ptr nonnull %_14)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %_14, ptr noundef nonnull align 8 dereferenceable(24) %division, i64 24, i1 false)
  %0 = getelementptr inbounds nuw i8, ptr %_14, i64 168
  store ptr %substitutions, ptr %0, align 8
  %1 = getelementptr inbounds nuw i8, ptr %_14, i64 176
  store ptr %relations, ptr %1, align 8
  %2 = getelementptr inbounds nuw i8, ptr %_14, i64 184
  store ptr %relations_from_graph, ptr %2, align 8
  %3 = getelementptr inbounds nuw i8, ptr %_14, i64 192
  store ptr %path_relations, ptr %3, align 8
  %4 = getelementptr inbounds nuw i8, ptr %_14, i64 24
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %4, ptr noundef nonnull align 8 dereferenceable(24) %extension_namespaces, i64 24, i1 false)
  %5 = getelementptr inbounds nuw i8, ptr %_14, i64 48
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %5, ptr noundef nonnull align 8 dereferenceable(24) %property_fn_namespaces, i64 24, i1 false)
  %6 = getelementptr inbounds nuw i8, ptr %_14, i64 72
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %6, ptr noundef nonnull align 8 dereferenceable(48) %standpoint_predicates, i64 48, i1 false)
  %7 = getelementptr inbounds nuw i8, ptr %_14, i64 200
  store ptr %xpath_regex.0, ptr %7, align 8
  %8 = getelementptr inbounds nuw i8, ptr %_14, i64 208
  store i64 %xpath_regex.1, ptr %8, align 8
  %9 = getelementptr inbounds nuw i8, ptr %_14, i64 144
  store ptr %self, ptr %9, align 8
  %10 = getelementptr inbounds nuw i8, ptr %_14, i64 120
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %10, ptr noundef nonnull align 8 dereferenceable(24) %aggregate_namespace, i64 24, i1 false)
  %11 = getelementptr inbounds nuw i8, ptr %_14, i64 152
  store ptr %query.0, ptr %11, align 8
  %12 = getelementptr inbounds nuw i8, ptr %_14, i64 160
  store i64 %query.1, ptr %12, align 8
  tail call void @llvm.experimental.noalias.scope.decl(metadata !107641)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !107644)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !107646)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !107649)
  call void @llvm.lifetime.start.p0(ptr nonnull %_26.i.i), !noalias !107651
  call void @llvm.lifetime.start.p0(ptr nonnull %result.i.i), !noalias !107651
  call void @llvm.lifetime.start.p0(ptr nonnull %_3.i.i), !noalias !107652
  %13 = load i64, ptr %_14, align 8, !range !3909, !alias.scope !107653, !noalias !107654, !noundef !3892
  %.not.i.i = icmp eq i64 %13, -1
  %14 = getelementptr inbounds nuw i8, ptr %_14, i64 8
  %_50.i.i = load ptr, ptr %14, align 8, !alias.scope !107653, !noalias !107654, !nonnull !3892
  %15 = getelementptr inbounds nuw i8, ptr %_14, i64 16
  %_49.i.i = load i64, ptr %15, align 8, !alias.scope !107653, !noalias !107654
  %_4.sroa.5.0.i.i = select i1 %.not.i.i, i64 undef, i64 %_49.i.i
  %_4.sroa.0.0.i.i = select i1 %.not.i.i, ptr null, ptr %_50.i.i
; invoke purrdf_native::py_store::env::division_policy
  invoke fastcc void @purrdf_native::py_store::env::division_policy(ptr noalias nofree noundef align 8 captures(address) dereferenceable(56) %_3.i.i, ptr noalias nofree noundef readonly captures(address, read_provenance) %_4.sroa.0.0.i.i, i64 %_4.sroa.5.0.i.i)
          to label %bb1.i.i unwind label %cleanup.i.i, !noalias !107652

bb47.i.i:                                         ; preds = %bb37.i.i, %cleanup9.i.i, %bb6.i.i.i, %cleanup1.body.i.i.i, %cleanup.i.i
  %_39.sroa.0.0.i.i = phi i1 [ true, %cleanup.i.i ], [ true, %bb37.i.i ], [ false, %cleanup9.i.i ], [ false, %bb6.i.i.i ], [ false, %cleanup1.body.i.i.i ]
  %_42.sroa.0.0.i.i = phi i8 [ %_42.sroa.0.1.i.i, %cleanup.i.i ], [ %_42.sroa.0.2.ph.i.i, %bb37.i.i ], [ 0, %cleanup9.i.i ], [ 0, %bb6.i.i.i ], [ 0, %cleanup1.body.i.i.i ]
  %.pn35.i.i = phi { ptr, i32 } [ %17, %cleanup.i.i ], [ %.pn33.ph.i.i, %bb37.i.i ], [ %148, %cleanup9.i.i ], [ %147, %bb6.i.i.i ], [ %eh.lpad-body.i.i.i, %cleanup1.body.i.i.i ]
  %16 = icmp sgt i64 %13, 0
  br i1 %16, label %bb2.i.i.i4.i.i.i.i.i, label %bb45.i.i

bb2.i.i.i4.i.i.i.i.i:                             ; preds = %bb47.i.i
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %_50.i.i, i64 noundef %13, i64 noundef range(i64 1, -9223372036854775807) 1) #77, !noalias !107655
  br label %bb45.i.i

cleanup.i.i:                                      ; preds = %bb62.i.i, %bb52.i.i, %start
  %_42.sroa.0.1.i.i = phi i8 [ %_42.sroa.0.5.i.i, %bb62.i.i ], [ 1, %bb52.i.i ], [ 1, %start ]
  %17 = landingpad { ptr, i32 }
          cleanup
  br label %bb47.i.i

bb1.i.i:                                          ; preds = %start
  %18 = load i32, ptr %_3.i.i, align 8, !range !8054, !noalias !107652, !noundef !3892
  %19 = trunc nuw i32 %18 to i1
  br i1 %19, label %bb51.i.i, label %bb52.i.i

bb51.i.i:                                         ; preds = %bb1.i.i
  %20 = getelementptr inbounds nuw i8, ptr %_3.i.i, i64 8
  %_53.sroa.0.0.copyload.i.i = load i32, ptr %20, align 8, !noalias !107652
  %_53.sroa.4.0..sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %_3.i.i, i64 12
  %_56.sroa.4.0..sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %_0, i64 12
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 4 dereferenceable(44) %_56.sroa.4.0..sroa_idx.i.i, ptr noundef nonnull align 4 dereferenceable(44) %_53.sroa.4.0..sroa_idx.i.i, i64 44, i1 false), !noalias !107653
  call void @llvm.lifetime.end.p0(ptr nonnull %_3.i.i), !noalias !107652
  %21 = getelementptr inbounds nuw i8, ptr %_0, i64 8
  store i32 %_53.sroa.0.0.copyload.i.i, ptr %21, align 8, !alias.scope !107654, !noalias !107653
  store i64 1, ptr %_0, align 8, !alias.scope !107654, !noalias !107653
  br label %bb61.i.i

bb52.i.i:                                         ; preds = %bb1.i.i
  %22 = getelementptr inbounds nuw i8, ptr %_3.i.i, i64 4
  %_52.sroa.0.0.copyload.i.i = load i64, ptr %22, align 4, !noalias !107652
  call void @llvm.lifetime.end.p0(ptr nonnull %_3.i.i), !noalias !107652
  call void @llvm.lifetime.start.p0(ptr nonnull %subs.i.i), !noalias !107652
  call void @llvm.lifetime.start.p0(ptr nonnull %_8.sroa.5.i.i)
  call void @llvm.lifetime.start.p0(ptr nonnull %_9.i.i), !noalias !107652
; invoke purrdf_native::py_store::quad_store::collect_substitutions
  invoke fastcc void @purrdf_native::py_store::quad_store::collect_substitutions(ptr noalias nofree noundef align 8 captures(none) dereferenceable(56) %_9.i.i, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable_or_null(8) %substitutions)
          to label %bb3.i.i unwind label %cleanup.i.i, !noalias !107652

bb3.i.i:                                          ; preds = %bb52.i.i
  %_58.i.i = load i64, ptr %_9.i.i, align 8, !range !6076, !noalias !107652, !noundef !3892
  %23 = trunc nuw i64 %_58.i.i to i1
  %24 = getelementptr inbounds nuw i8, ptr %_9.i.i, i64 8
  br i1 %23, label %bb53.i.i, label %bb54.i.i

bb53.i.i:                                         ; preds = %bb3.i.i
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %_8.sroa.5.i.i, ptr noundef nonnull align 8 dereferenceable(48) %24, i64 48, i1 false), !noalias !107652
  call void @llvm.lifetime.end.p0(ptr nonnull %_9.i.i), !noalias !107652
  %25 = getelementptr inbounds nuw i8, ptr %_0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %25, ptr noundef nonnull align 8 dereferenceable(48) %_8.sroa.5.i.i, i64 48, i1 false), !noalias !107653
  store i64 1, ptr %_0, align 8, !alias.scope !107654, !noalias !107653
  call void @llvm.lifetime.end.p0(ptr nonnull %_8.sroa.5.i.i)
  call void @llvm.lifetime.end.p0(ptr nonnull %subs.i.i), !noalias !107652
  br label %bb61.i.i

bb54.i.i:                                         ; preds = %bb3.i.i
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %_8.sroa.5.i.i, ptr noundef nonnull align 8 dereferenceable(24) %24, i64 24, i1 false), !noalias !107652
  call void @llvm.lifetime.end.p0(ptr nonnull %_9.i.i), !noalias !107652
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %subs.i.i, ptr noundef nonnull align 8 dereferenceable(24) %_8.sroa.5.i.i, i64 24, i1 false), !noalias !107652
  call void @llvm.lifetime.end.p0(ptr nonnull %_8.sroa.5.i.i)
  call void @llvm.lifetime.start.p0(ptr nonnull %specs.i.i), !noalias !107652
  call void @llvm.lifetime.start.p0(ptr nonnull %_14.sroa.5.i.i)
  call void @llvm.lifetime.start.p0(ptr nonnull %_15.i.i), !noalias !107652
; invoke purrdf_native::py_store::query::collect_relations
  invoke fastcc void @purrdf_native::py_store::query::collect_relations(ptr noalias nofree noundef align 8 captures(none) dereferenceable(56) %_15.i.i, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable_or_null(8) %relations, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable_or_null(8) %relations_from_graph, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable_or_null(8) %path_relations)
          to label %bb4.i.i unwind label %cleanup7.i.i, !noalias !107652

cleanup7.i.i:                                     ; preds = %bb54.i.i
  %26 = landingpad { ptr, i32 }
          cleanup
  br label %bb37.i.i

bb4.i.i:                                          ; preds = %bb54.i.i
  %_65.i.i = load i64, ptr %_15.i.i, align 8, !range !6076, !noalias !107652, !noundef !3892
  %27 = trunc nuw i64 %_65.i.i to i1
  %28 = getelementptr inbounds nuw i8, ptr %_15.i.i, i64 8
  br i1 %27, label %bb55.i.i, label %bb56.i.i

bb55.i.i:                                         ; preds = %bb4.i.i
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %_14.sroa.5.i.i, ptr noundef nonnull align 8 dereferenceable(48) %28, i64 48, i1 false), !noalias !107652
  call void @llvm.lifetime.end.p0(ptr nonnull %_15.i.i), !noalias !107652
  %29 = getelementptr inbounds nuw i8, ptr %_0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %29, ptr noundef nonnull align 8 dereferenceable(48) %_14.sroa.5.i.i, i64 48, i1 false), !noalias !107653
  store i64 1, ptr %_0, align 8, !alias.scope !107654, !noalias !107653
  call void @llvm.lifetime.end.p0(ptr nonnull %_14.sroa.5.i.i)
  br label %bb62.i.i

bb56.i.i:                                         ; preds = %bb4.i.i
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %_14.sroa.5.i.i, ptr noundef nonnull align 8 dereferenceable(24) %28, i64 24, i1 false), !noalias !107652
  call void @llvm.lifetime.end.p0(ptr nonnull %_15.i.i), !noalias !107652
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %specs.i.i, ptr noundef nonnull align 8 dereferenceable(24) %_14.sroa.5.i.i, i64 24, i1 false), !noalias !107652
  call void @llvm.lifetime.end.p0(ptr nonnull %_14.sroa.5.i.i)
  call void @llvm.lifetime.start.p0(ptr nonnull %config.sroa.0.i.i)
  %_22.i.sroa.0.0.copyload.i = load i64, ptr %4, align 8, !alias.scope !107644, !noalias !107654
  %_22.i.sroa.5.0..sroa_idx.i = getelementptr inbounds nuw i8, ptr %_14, i64 32
  %_22.i.sroa.5.0.copyload.i = load ptr, ptr %_22.i.sroa.5.0..sroa_idx.i, align 8, !alias.scope !107644, !noalias !107654
  %_22.i.sroa.6.0..sroa_idx.i = getelementptr inbounds nuw i8, ptr %_14, i64 40
  %_22.i.sroa.6.0.copyload.i = load i64, ptr %_22.i.sroa.6.0..sroa_idx.i, align 8, !alias.scope !107644, !noalias !107654
  %_23.i.sroa.0.0.copyload.i = load i64, ptr %5, align 8, !alias.scope !107644, !noalias !107654
  %_23.i.sroa.5.0..sroa_idx.i = getelementptr inbounds nuw i8, ptr %_14, i64 56
  %_23.i.sroa.5.0.copyload.i = load ptr, ptr %_23.i.sroa.5.0..sroa_idx.i, align 8, !alias.scope !107644, !noalias !107654
  %_23.i.sroa.6.0..sroa_idx.i = getelementptr inbounds nuw i8, ptr %_14, i64 64
  %_23.i.sroa.6.0.copyload.i = load i64, ptr %_23.i.sroa.6.0..sroa_idx.i, align 8, !alias.scope !107644, !noalias !107654
  %_24.i.sroa.0.0.copyload.i = load i64, ptr %6, align 8, !alias.scope !107644, !noalias !107654
  %_24.i.sroa.5.0..sroa_idx.i = getelementptr inbounds nuw i8, ptr %_14, i64 80
  %_24.i.sroa.5.0.copyload.i = load ptr, ptr %_24.i.sroa.5.0..sroa_idx.i, align 8, !alias.scope !107644, !noalias !107654
  %_24.i.sroa.628.0..sroa_idx.i = getelementptr inbounds nuw i8, ptr %_14, i64 96
  %_24.i.sroa.628.0.copyload.i = load i64, ptr %_24.i.sroa.628.0..sroa_idx.i, align 8, !alias.scope !107644, !noalias !107654
  %_24.i.sroa.7.0..sroa_idx.i = getelementptr inbounds nuw i8, ptr %_14, i64 104
  %_24.i.sroa.7.0.copyload.i = load ptr, ptr %_24.i.sroa.7.0..sroa_idx.i, align 8, !alias.scope !107644, !noalias !107654
; invoke purrdf_native::xpath_regex::selection
  invoke fastcc void @purrdf_native::xpath_regex::selection(ptr noalias nofree noundef align 8 captures(none) dereferenceable(72) %_26.i.i, ptr noalias nofree noundef readonly captures(address, read_provenance) %xpath_regex.0, i64 %xpath_regex.1)
          to label %bb5.i.i unwind label %cleanup8.i.i, !noalias !107652

cleanup8.i.i:                                     ; preds = %bb56.i.i
  %30 = landingpad { ptr, i32 }
          cleanup
; call core::ptr::drop_glue::<core::option::Option<purrdf_sparql_results::model::ProvenanceNamespace>>
  call fastcc void @core::ptr::drop_glue::<core::option::Option<purrdf_sparql_results::model::ProvenanceNamespace>>(ptr noalias nofree noundef readonly align 8 dereferenceable(48) %6) #79, !noalias !107654
; call core::ptr::drop_glue::<core::option::Option<alloc::vec::Vec<alloc::string::String>>>
  call fastcc void @core::ptr::drop_glue::<core::option::Option<alloc::vec::Vec<alloc::string::String>>>(ptr noalias nofree noundef readonly align 8 dereferenceable(24) %5) #79, !noalias !107654
; call core::ptr::drop_glue::<core::option::Option<alloc::vec::Vec<alloc::string::String>>>
  call fastcc void @core::ptr::drop_glue::<core::option::Option<alloc::vec::Vec<alloc::string::String>>>(ptr noalias nofree noundef readonly align 8 dereferenceable(24) %4) #79, !noalias !107654
; invoke core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>>(ptr noalias nofree noundef align 8 dereferenceable(24) %specs.i.i) #79
          to label %bb37.i.i unwind label %terminate.i.i, !noalias !107652

bb5.i.i:                                          ; preds = %bb56.i.i
  %31 = load i8, ptr %_26.i.i, align 8, !range !16607, !noalias !107652, !noundef !3892
  %32 = icmp eq i8 %31, -1
  br i1 %32, label %bb57.i.i, label %bb58.i.i

bb57.i.i:                                         ; preds = %bb5.i.i
  %33 = getelementptr inbounds nuw i8, ptr %_26.i.i, i64 8
  %34 = getelementptr inbounds nuw i8, ptr %_0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %34, ptr noundef nonnull align 8 dereferenceable(48) %33, i64 48, i1 false), !noalias !107653
  store i64 1, ptr %_0, align 8, !alias.scope !107654, !noalias !107653
  switch i64 %_24.i.sroa.0.0.copyload.i, label %bb2.i.i.i4.i.i.i.i.i.i [
    i64 -1, label %bb8.i.i
    i64 0, label %bb4.i.i.i.i
  ]

bb2.i.i.i4.i.i.i.i.i.i:                           ; preds = %bb57.i.i
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %_24.i.sroa.5.0.copyload.i) ]
; call __rustc::__rust_dealloc
  tail call void @__rustc::__rust_dealloc(ptr noundef nonnull %_24.i.sroa.5.0.copyload.i, i64 noundef %_24.i.sroa.0.0.copyload.i, i64 noundef range(i64 1, -9223372036854775807) 1) #77, !noalias !107660
  br label %bb4.i.i.i.i

bb4.i.i.i.i:                                      ; preds = %bb2.i.i.i4.i.i.i.i.i.i, %bb57.i.i
  %35 = icmp eq i64 %_24.i.sroa.628.0.copyload.i, 0
  br i1 %35, label %bb8.i.i, label %bb2.i.i.i4.i.i6.i.i.i.i

bb2.i.i.i4.i.i6.i.i.i.i:                          ; preds = %bb4.i.i.i.i
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %_24.i.sroa.7.0.copyload.i) ]
; call __rustc::__rust_dealloc
  tail call void @__rustc::__rust_dealloc(ptr noundef nonnull %_24.i.sroa.7.0.copyload.i, i64 noundef %_24.i.sroa.628.0.copyload.i, i64 noundef range(i64 1, -9223372036854775807) 1) #77, !noalias !107667
  br label %bb8.i.i

bb58.i.i:                                         ; preds = %bb5.i.i
  %_73.sroa.4.0._26.sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %_26.i.i, i64 1
  %_73.sroa.5.0._26.sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %_26.i.i, i64 56
  %config.sroa.10.0..sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %_33.i.i, i64 224
  call void @llvm.lifetime.start.p0(ptr nonnull %_33.i.i), !noalias !107652
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %config.sroa.10.0..sroa_idx.i.i, ptr noundef nonnull align 8 dereferenceable(16) %_73.sroa.5.0._26.sroa_idx.i.i, i64 16, i1 false), !noalias !107652
  %config.sroa.9.0..sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %_33.i.i, i64 169
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(55) %config.sroa.9.0..sroa_idx.i.i, ptr noundef nonnull align 1 dereferenceable(55) %_73.sroa.4.0._26.sroa_idx.i.i, i64 55, i1 false), !noalias !107652
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %config.sroa.0.i.i, ptr noundef nonnull align 8 dereferenceable(24) %extension_namespaces, i64 24, i1 false)
  %config.sroa.0.24..sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %config.sroa.0.i.i, i64 24
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %config.sroa.0.24..sroa_idx.i.i, ptr noundef nonnull align 8 dereferenceable(24) %property_fn_namespaces, i64 24, i1 false)
  %config.sroa.0.48..sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %config.sroa.0.i.i, i64 48
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %config.sroa.0.48..sroa_idx.i.i, ptr noundef nonnull align 8 dereferenceable(48) %standpoint_predicates, i64 48, i1 false)
  call void @llvm.lifetime.start.p0(ptr nonnull %_31.sroa.6.sroa.0.i.i)
  call void @llvm.lifetime.start.p0(ptr nonnull %_32.sroa.10.i.i)
  %36 = getelementptr inbounds nuw i8, ptr %_33.i.i, i64 248
  store ptr %self, ptr %36, align 8, !noalias !107652
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %_33.i.i, ptr noundef nonnull align 8 dereferenceable(24) %specs.i.i, i64 24, i1 false), !noalias !107652
  %37 = getelementptr inbounds nuw i8, ptr %_33.i.i, i64 48
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %37, ptr noundef nonnull align 8 dereferenceable(24) %aggregate_namespace, i64 24, i1 false)
  %38 = getelementptr inbounds nuw i8, ptr %_33.i.i, i64 72
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(96) %38, ptr noundef nonnull align 8 dereferenceable(96) %config.sroa.0.i.i, i64 96, i1 false), !noalias !107652
  %config.sroa.8.0..sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %_33.i.i, i64 168
  store i8 %31, ptr %config.sroa.8.0..sroa_idx.i.i, align 8, !noalias !107652
  %39 = getelementptr inbounds nuw i8, ptr %_33.i.i, i64 256
  store ptr %query.0, ptr %39, align 8, !noalias !107652
  %40 = getelementptr inbounds nuw i8, ptr %_33.i.i, i64 264
  store i64 %query.1, ptr %40, align 8, !noalias !107652
  %41 = getelementptr inbounds nuw i8, ptr %_33.i.i, i64 24
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %41, ptr noundef nonnull align 8 dereferenceable(24) %subs.i.i, i64 24, i1 false), !noalias !107652
  %42 = getelementptr inbounds nuw i8, ptr %_33.i.i, i64 240
  store i64 %_52.sroa.0.0.copyload.i.i, ptr %42, align 8, !noalias !107652
  tail call void @llvm.experimental.noalias.scope.decl(metadata !107670)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !107673)
  call void @llvm.lifetime.start.p0(ptr nonnull %_guard.i.i.i), !noalias !107675
; invoke <pyo3::internal::state::SuspendAttach>::new
  %43 = invoke { i64, ptr } @<pyo3::internal::state::SuspendAttach>::new()
          to label %bb1.i.i.i unwind label %bb6.i.i.i, !noalias !107675

bb1.i.i.i:                                        ; preds = %bb58.i.i
  %44 = extractvalue { i64, ptr } %43, 0
  %45 = extractvalue { i64, ptr } %43, 1
  store i64 %44, ptr %_guard.i.i.i, align 8, !noalias !107675
  %46 = getelementptr inbounds nuw i8, ptr %_guard.i.i.i, i64 8
  store ptr %45, ptr %46, align 8, !noalias !107675
  tail call void @llvm.experimental.noalias.scope.decl(metadata !107676)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !107679)
  call void @llvm.lifetime.start.p0(ptr nonnull %parser_options.i.i.i.i), !noalias !107675
  call void @llvm.lifetime.start.p0(ptr nonnull %dataset.i.i.i.i), !noalias !107681
  call void @llvm.lifetime.start.p0(ptr nonnull %_5.i.i.i.i), !noalias !107681
; invoke <purrdf_core::ir::mutable::MutableDataset>::freeze
  invoke void @<purrdf_core::ir::mutable::MutableDataset>::freeze(ptr noalias nofree noundef nonnull sret([96 x i8]) align 8 captures(none) dereferenceable(96) %_5.i.i.i.i, ptr noundef nonnull align 8 %self)
          to label %bb1.i.i.i.i unwind label %bb40.thread127.i.i.i.i, !noalias !107681

bb40.thread127.i.i.i.i:                           ; preds = %bb8.i.i.i.i.i, %bb1.i.i.i
  %lpad.thr_comm.i.i.i.i = landingpad { ptr, i32 }
          cleanup
  br label %bb39.i.i.i.i

bb40.i.i.i.i:                                     ; preds = %bb2.i.i97.i.i.i.i, %bb2.i.i75.i.i.i.i
  %_40.sroa.0.1.ph.i.i.i.i = phi i8 [ %_40.sroa.0.7.i.i.i.i, %bb2.i.i97.i.i.i.i ], [ 0, %bb2.i.i75.i.i.i.i ]
  %lpad.thr_comm.split-lp.i.i.i.i = landingpad { ptr, i32 }
          cleanup
  br label %bb38.i.i.i.i

bb1.i.i.i.i:                                      ; preds = %bb1.i.i.i
  %47 = load i64, ptr %_5.i.i.i.i, align 8, !range !3909, !noalias !107681, !noundef !3892
  %.not.i.i.i.i = icmp eq i64 %47, -1
  br i1 %.not.i.i.i.i, label %bb42.i.i.i.i, label %bb41.i.i.i.i

bb41.i.i.i.i:                                     ; preds = %bb1.i.i.i.i
  call void @llvm.lifetime.start.p0(ptr nonnull %_47.i.i.i.i), !noalias !107681
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(96) %_47.i.i.i.i, ptr noundef nonnull align 8 dereferenceable(96) %_5.i.i.i.i, i64 96, i1 false), !noalias !107681
  call void @llvm.lifetime.start.p0(ptr nonnull %_3.i.i.i.i.i), !noalias !107681
  call void @llvm.lifetime.start.p0(ptr nonnull %args.i.i.i.i.i), !noalias !107682
  store ptr %_47.i.i.i.i, ptr %args.i.i.i.i.i, align 8, !noalias !107682
  %_7.sroa.4.0..sroa_idx.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %args.i.i.i.i.i, i64 8
  store ptr @<purrdf_core::diagnostic::RdfDiagnostic as core::fmt::Display>::fmt, ptr %_7.sroa.4.0..sroa_idx.i.i.i.i.i, align 8, !noalias !107682
; invoke alloc::fmt::format::format_inner
  invoke void @alloc::fmt::format::format_inner(ptr noalias nofree noundef nonnull sret([24 x i8]) align 8 captures(none) dereferenceable(24) %_3.i.i.i.i.i, ptr noundef nonnull @alloc_0e5f90e3dc675d538220deef7d17e145, ptr noundef nonnull %args.i.i.i.i.i)
          to label %bb4.i.i.i.i.i unwind label %cleanup.i.i.i.i.i, !noalias !107686

cleanup.i.i.i.i.i:                                ; preds = %bb41.i.i.i.i
  %48 = landingpad { ptr, i32 }
          cleanup
  br label %cleanup.body.i.i.i.i.i

cleanup.body.i.i.i.i.i:                           ; preds = %bb2.i.i.i4.i.i.i.i.i.i.i.i, %cleanup.i.i.i.i.i.i, %cleanup.i.i.i.i.i
  %eh.lpad-body.i.i.i.i.i = phi { ptr, i32 } [ %48, %cleanup.i.i.i.i.i ], [ %51, %bb2.i.i.i4.i.i.i.i.i.i.i.i ], [ %51, %cleanup.i.i.i.i.i.i ]
; invoke core::ptr::drop_glue::<purrdf_core::diagnostic::RdfDiagnostic>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_core::diagnostic::RdfDiagnostic>(ptr noalias nofree noundef nonnull align 8 dereferenceable(96) %_47.i.i.i.i) #79
          to label %bb39.i.i.i.i unwind label %terminate.i.i.i.i.i, !noalias !107686

bb4.i.i.i.i.i:                                    ; preds = %bb41.i.i.i.i
  call void @llvm.lifetime.end.p0(ptr nonnull %args.i.i.i.i.i), !noalias !107682
  %_29.sroa.0.0.copyload.i.i.i.i.i = load i64, ptr %_3.i.i.i.i.i, align 8, !noalias !107682
  %_29.sroa.5.0._3.sroa_idx.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %_3.i.i.i.i.i, i64 8
  %_29.sroa.5.0.copyload.i.i.i.i.i = load ptr, ptr %_29.sroa.5.0._3.sroa_idx.i.i.i.i.i, align 8, !noalias !107682
  %_29.sroa.6.0._3.sroa_idx.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %_3.i.i.i.i.i, i64 16
  %_29.sroa.6.0.copyload.i.i.i.i.i = load i64, ptr %_29.sroa.6.0._3.sroa_idx.i.i.i.i.i, align 8, !noalias !107682
; call __rustc::__rust_no_alloc_shim_is_unstable_v2
  call void @__rustc::__rust_no_alloc_shim_is_unstable_v2() #77, !noalias !107687
; call __rustc::__rust_alloc
  %49 = call noundef align 8 dereferenceable_or_null(24) ptr @__rustc::__rust_alloc(i64 noundef 24, i64 noundef range(i64 1, -9223372036854775807) 8) #77, !noalias !107687
  %50 = icmp eq ptr %49, null
  br i1 %50, label %bb2.i5.i.i.i.i.i, label %bb8.i.i.i.i.i, !prof !4833

bb2.i5.i.i.i.i.i:                                 ; preds = %bb4.i.i.i.i.i
; invoke alloc::alloc::handle_alloc_error
  invoke void @alloc::alloc::handle_alloc_error(i64 noundef 8, i64 noundef 24) #80
          to label %.noexc.i.i.i.i.i unwind label %cleanup.i.i.i.i.i.i, !noalias !107686

.noexc.i.i.i.i.i:                                 ; preds = %bb2.i5.i.i.i.i.i
  unreachable

cleanup.i.i.i.i.i.i:                              ; preds = %bb2.i5.i.i.i.i.i
  %51 = landingpad { ptr, i32 }
          cleanup
  %52 = icmp eq i64 %_29.sroa.0.0.copyload.i.i.i.i.i, 0
  br i1 %52, label %cleanup.body.i.i.i.i.i, label %bb2.i.i.i4.i.i.i.i.i.i.i.i

bb2.i.i.i4.i.i.i.i.i.i.i.i:                       ; preds = %cleanup.i.i.i.i.i.i
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %_29.sroa.5.0.copyload.i.i.i.i.i) ]
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %_29.sroa.5.0.copyload.i.i.i.i.i, i64 noundef %_29.sroa.0.0.copyload.i.i.i.i.i, i64 noundef range(i64 1, -9223372036854775807) 1) #77, !noalias !107690
  br label %cleanup.body.i.i.i.i.i

bb8.i.i.i.i.i:                                    ; preds = %bb4.i.i.i.i.i
  store i64 %_29.sroa.0.0.copyload.i.i.i.i.i, ptr %49, align 8, !noalias !107686
  %_29.sroa.5.0..sroa_idx.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %49, i64 8
  store ptr %_29.sroa.5.0.copyload.i.i.i.i.i, ptr %_29.sroa.5.0..sroa_idx.i.i.i.i.i, align 8, !noalias !107686
  %_29.sroa.6.0..sroa_idx.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %49, i64 16
  store i64 %_29.sroa.6.0.copyload.i.i.i.i.i, ptr %_29.sroa.6.0..sroa_idx.i.i.i.i.i, align 8, !noalias !107686
; invoke core::ptr::drop_glue::<purrdf_core::diagnostic::RdfDiagnostic>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_core::diagnostic::RdfDiagnostic>(ptr noalias nofree noundef nonnull align 8 dereferenceable(96) %_47.i.i.i.i)
          to label %bb43.i.i.i.i unwind label %bb40.thread127.i.i.i.i, !noalias !107681

terminate.i.i.i.i.i:                              ; preds = %cleanup.body.i.i.i.i.i
  %53 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #75, !noalias !107686
  unreachable

bb42.i.i.i.i:                                     ; preds = %bb1.i.i.i.i
  %54 = getelementptr inbounds nuw i8, ptr %_5.i.i.i.i, i64 8
  %_44.i.i.i.i = load ptr, ptr %54, align 8, !noalias !107681, !nonnull !3892, !noundef !3892
  call void @llvm.lifetime.end.p0(ptr nonnull %_5.i.i.i.i), !noalias !107681
  store ptr %_44.i.i.i.i, ptr %dataset.i.i.i.i, align 8, !noalias !107681
  call void @llvm.lifetime.start.p0(ptr nonnull %registry.i.i.i.i), !noalias !107681
  call void @llvm.lifetime.start.p0(ptr nonnull %_9.sroa.5.i.i.sroa.0.i.i)
  call void @llvm.lifetime.start.p0(ptr nonnull %_10.i.i.i.i), !noalias !107681
  %_12.i.i.i.i = getelementptr inbounds nuw i8, ptr %_44.i.i.i.i, i64 16
; invoke purrdf_native::py_store::query::build_relations
  invoke fastcc void @purrdf_native::py_store::query::build_relations(ptr noalias nofree noundef align 8 captures(none) dereferenceable(80) %_10.i.i.i.i, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(272) %_33.i.i, ptr noundef nonnull align 8 %_12.i.i.i.i)
          to label %bb3.i.i.i.i unwind label %cleanup5.i.i.i.i, !noalias !107695

bb21.i.i.i.i:                                     ; preds = %cleanup.i.i85.i.i.i.i, %cleanup.i.i68.i.i.i.i, %bb20.i.i.i.i, %cleanup5.i.i.i.i
  %_40.sroa.0.2.i.i.i.i = phi i8 [ %_40.sroa.0.4.i.i.i.i, %bb20.i.i.i.i ], [ 0, %cleanup.i.i68.i.i.i.i ], [ %_40.sroa.0.3.i.i.i.i, %cleanup5.i.i.i.i ], [ 0, %cleanup.i.i85.i.i.i.i ]
  %.pn33.i.i.i.i = phi { ptr, i32 } [ %.pn31.i.i.i.i, %bb20.i.i.i.i ], [ %115, %cleanup.i.i68.i.i.i.i ], [ %56, %cleanup5.i.i.i.i ], [ %120, %cleanup.i.i85.i.i.i.i ]
  call void @llvm.experimental.noalias.scope.decl(metadata !107696)
  call void @llvm.experimental.noalias.scope.decl(metadata !107699)
  %_10.i.i.i.i.i.i = load ptr, ptr %dataset.i.i.i.i, align 8, !alias.scope !107702, !noalias !107681, !nonnull !3892, !noundef !3892
  %_2.i.i.i.i.i.i = atomicrmw sub ptr %_10.i.i.i.i.i.i, i64 1 release, align 8, !noalias !107703
  %55 = icmp eq i64 %_2.i.i.i.i.i.i, 1
  br i1 %55, label %bb2.i.i.i.i.i.i, label %bb38.i.i.i.i

bb2.i.i.i.i.i.i:                                  ; preds = %bb21.i.i.i.i
  fence acquire
; invoke <alloc::sync::Arc<purrdf_core::ir::dataset::RdfDataset>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_core::ir::dataset::RdfDataset>>::drop_slow(ptr noalias nofree noundef nonnull align 8 dereferenceable(8) %dataset.i.i.i.i) #81
          to label %bb38.i.i.i.i unwind label %terminate.i.i.i.i, !noalias !107681

cleanup5.i.i.i.i:                                 ; preds = %core::ptr::drop_glue::<purrdf_sparql_eval::property_fn::PropertyFunctionRegistry> (.exit.i88.i.i.i.i), %core::ptr::drop_glue::<purrdf_sparql_eval::property_fn::PropertyFunctionRegistry> (.exit.i.i.i.i.i), %bb42.i.i.i.i
  %_40.sroa.0.3.i.i.i.i = phi i8 [ 0, %core::ptr::drop_glue::<purrdf_sparql_eval::property_fn::PropertyFunctionRegistry> (.exit.i88.i.i.i.i) ], [ 0, %core::ptr::drop_glue::<purrdf_sparql_eval::property_fn::PropertyFunctionRegistry> (.exit.i.i.i.i.i) ], [ 1, %bb42.i.i.i.i ]
  %56 = landingpad { ptr, i32 }
          cleanup
  br label %bb21.i.i.i.i

bb3.i.i.i.i:                                      ; preds = %bb42.i.i.i.i
  %_56.i.i.i.i = load i64, ptr %_10.i.i.i.i, align 8, !range !6076, !noalias !107681, !noundef !3892
  %57 = trunc nuw i64 %_56.i.i.i.i to i1
  %58 = getelementptr inbounds nuw i8, ptr %_10.i.i.i.i, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %_9.sroa.5.i.i.sroa.0.i.i, ptr noundef nonnull align 8 dereferenceable(16) %58, i64 16, i1 false), !noalias !107681
  %_9.sroa.5.i.i.sroa.7.0..sroa_idx191.i.i = getelementptr inbounds nuw i8, ptr %_10.i.i.i.i, i64 24
  %_9.sroa.5.i.i.sroa.7.0.copyload192.i.i = load i64, ptr %_9.sroa.5.i.i.sroa.7.0..sroa_idx191.i.i, align 8, !noalias !107681
  %_9.sroa.5.i.i.sroa.8.0..sroa_idx194.i.i = getelementptr inbounds nuw i8, ptr %_10.i.i.i.i, i64 32
  %_9.sroa.5.i.i.sroa.8.0.copyload195.i.i = load ptr, ptr %_9.sroa.5.i.i.sroa.8.0..sroa_idx194.i.i, align 8, !noalias !107681
  %_9.sroa.5.i.i.sroa.9.0..sroa_idx197.i.i = getelementptr inbounds nuw i8, ptr %_10.i.i.i.i, i64 40
  %_9.sroa.5.i.i.sroa.9.0.copyload198.i.i = load ptr, ptr %_9.sroa.5.i.i.sroa.9.0..sroa_idx197.i.i, align 8, !noalias !107681
  %_9.sroa.5.i.i.sroa.10.0..sroa_idx200.i.i = getelementptr inbounds nuw i8, ptr %_10.i.i.i.i, i64 48
  %_9.sroa.5.i.i.sroa.10.0.copyload201.i.i = load i32, ptr %_9.sroa.5.i.i.sroa.10.0..sroa_idx200.i.i, align 8, !noalias !107681
  %_9.sroa.5.i.i.sroa.11.0..sroa_idx203.i.i = getelementptr inbounds nuw i8, ptr %_10.i.i.i.i, i64 52
  %_9.sroa.5.i.i.sroa.11.0.copyload204.i.i = load i32, ptr %_9.sroa.5.i.i.sroa.11.0..sroa_idx203.i.i, align 4, !noalias !107681
  br i1 %57, label %bb44.i.i.i.i, label %bb45.i.i.i.i

bb44.i.i.i.i:                                     ; preds = %bb3.i.i.i.i
  call void @llvm.lifetime.end.p0(ptr nonnull %_10.i.i.i.i), !noalias !107681
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %_32.sroa.10.i.i, ptr noundef nonnull align 8 dereferenceable(16) %_9.sroa.5.i.i.sroa.0.i.i, i64 16, i1 false), !noalias !107704
  call void @llvm.lifetime.end.p0(ptr nonnull %_9.sroa.5.i.i.sroa.0.i.i)
  br label %bb15.i.i.i.i

bb45.i.i.i.i:                                     ; preds = %bb3.i.i.i.i
  %_9.sroa.5.i.i.sroa.12.0..sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %_10.i.i.i.i, i64 56
  %_9.sroa.5.i.i.sroa.12.0.registry.i.i.sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %registry.i.i.i.i, i64 48
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %_9.sroa.5.i.i.sroa.12.0.registry.i.i.sroa_idx.i.i, ptr noundef nonnull align 8 dereferenceable(24) %_9.sroa.5.i.i.sroa.12.0..sroa_idx.i.i, i64 24, i1 false), !noalias !107681
  call void @llvm.lifetime.end.p0(ptr nonnull %_10.i.i.i.i), !noalias !107681
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %registry.i.i.i.i, ptr noundef nonnull align 8 dereferenceable(16) %_9.sroa.5.i.i.sroa.0.i.i, i64 16, i1 false), !noalias !107681
  %_9.sroa.5.i.i.sroa.7.0.registry.i.i.sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %registry.i.i.i.i, i64 16
  store i64 %_9.sroa.5.i.i.sroa.7.0.copyload192.i.i, ptr %_9.sroa.5.i.i.sroa.7.0.registry.i.i.sroa_idx.i.i, align 8, !noalias !107681
  %_9.sroa.5.i.i.sroa.8.0.registry.i.i.sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %registry.i.i.i.i, i64 24
  store ptr %_9.sroa.5.i.i.sroa.8.0.copyload195.i.i, ptr %_9.sroa.5.i.i.sroa.8.0.registry.i.i.sroa_idx.i.i, align 8, !noalias !107681
  %_9.sroa.5.i.i.sroa.9.0.registry.i.i.sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %registry.i.i.i.i, i64 32
  store ptr %_9.sroa.5.i.i.sroa.9.0.copyload198.i.i, ptr %_9.sroa.5.i.i.sroa.9.0.registry.i.i.sroa_idx.i.i, align 8, !noalias !107681
  %_9.sroa.5.i.i.sroa.10.0.registry.i.i.sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %registry.i.i.i.i, i64 40
  store i32 %_9.sroa.5.i.i.sroa.10.0.copyload201.i.i, ptr %_9.sroa.5.i.i.sroa.10.0.registry.i.i.sroa_idx.i.i, align 8, !noalias !107681
  %_9.sroa.5.i.i.sroa.11.0.registry.i.i.sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %registry.i.i.i.i, i64 44
  store i32 %_9.sroa.5.i.i.sroa.11.0.copyload204.i.i, ptr %_9.sroa.5.i.i.sroa.11.0.registry.i.i.sroa_idx.i.i, align 4, !noalias !107681
  call void @llvm.lifetime.end.p0(ptr nonnull %_9.sroa.5.i.i.sroa.0.i.i)
  call void @llvm.lifetime.start.p0(ptr nonnull %aggregates.i.i.i.i), !noalias !107681
  %59 = load i64, ptr %37, align 8, !range !3909, !alias.scope !107705, !noalias !107695, !noundef !3892
  %.not23.i.i.i.i = icmp eq i64 %59, -1
  %60 = getelementptr inbounds nuw i8, ptr %_33.i.i, i64 56
  %_67.i.i.i.i = load ptr, ptr %60, align 8, !alias.scope !107705, !noalias !107695, !nonnull !3892
  %61 = getelementptr inbounds nuw i8, ptr %_33.i.i, i64 64
  %_66.i.i.i.i = load i64, ptr %61, align 8, !alias.scope !107705, !noalias !107695
  %_16.sroa.5.0.i.i.i.i = select i1 %.not23.i.i.i.i, i64 undef, i64 %_66.i.i.i.i
  %_16.sroa.0.0.i.i.i.i = select i1 %.not23.i.i.i.i, ptr null, ptr %_67.i.i.i.i
; invoke purrdf_validate::query::statistical_aggregates
  invoke void @purrdf_validate::query::statistical_aggregates(ptr noalias nofree noundef nonnull sret([40 x i8]) align 8 captures(none) dereferenceable(40) %aggregates.i.i.i.i, ptr noalias nofree noundef readonly captures(address, read_provenance) %_16.sroa.0.0.i.i.i.i, i64 %_16.sroa.5.0.i.i.i.i)
          to label %bb4.i.i49.i.i unwind label %cleanup6.i.i.i.i, !noalias !107681

bb20.i.i.i.i:                                     ; preds = %bb2.i.i.i.i.i, %bb19.i.i.i.i, %cleanup6.i.i.i.i
  %_40.sroa.0.4.i.i.i.i = phi i8 [ %_40.sroa.0.5.i.i.i.i, %cleanup6.i.i.i.i ], [ %_40.sroa.0.6.i.i.i.i, %bb2.i.i.i.i.i ], [ %_40.sroa.0.6.i.i.i.i, %bb19.i.i.i.i ]
  %.pn31.i.i.i.i = phi { ptr, i32 } [ %62, %cleanup6.i.i.i.i ], [ %.pn28.pn.i.i.i.i, %bb2.i.i.i.i.i ], [ %.pn28.pn.i.i.i.i, %bb19.i.i.i.i ]
; invoke core::ptr::drop_glue::<core::option::Option<purrdf_sparql_eval::property_fn::PropertyFunctionRegistry>>
  invoke fastcc void @core::ptr::drop_glue::<core::option::Option<purrdf_sparql_eval::property_fn::PropertyFunctionRegistry>>(ptr noalias nofree noundef align 8 dereferenceable(72) %registry.i.i.i.i) #79
          to label %bb21.i.i.i.i unwind label %terminate.i.i.i.i, !noalias !107681

cleanup6.i.i.i.i:                                 ; preds = %bb2.i80.i.i.i.i, %bb2.i63.i.i.i.i, %bb45.i.i.i.i
  %_40.sroa.0.5.i.i.i.i = phi i8 [ 0, %bb2.i80.i.i.i.i ], [ 0, %bb2.i63.i.i.i.i ], [ 1, %bb45.i.i.i.i ]
  %62 = landingpad { ptr, i32 }
          cleanup
  br label %bb20.i.i.i.i

bb4.i.i49.i.i:                                    ; preds = %bb45.i.i.i.i
  tail call void @llvm.experimental.noalias.scope.decl(metadata !107706)
  call void @llvm.lifetime.start.p0(ptr nonnull %_2.i.i.i.i.i), !noalias !107709
  %63 = load i64, ptr %38, align 8, !range !3909, !alias.scope !107711, !noalias !107712, !noundef !3892
  %.not.i.i.i.i.i = icmp eq i64 %63, -1
  br i1 %.not.i.i.i.i.i, label %bb4.i43.i.i.i.i, label %bb5.i.i.i.i.i

bb5.i.i.i.i.i:                                    ; preds = %bb4.i.i49.i.i
  %64 = getelementptr inbounds nuw i8, ptr %_33.i.i, i64 80
  %config.val.i.i.i.i.i = load ptr, ptr %64, align 8, !alias.scope !107711, !noalias !107712, !nonnull !3892, !noundef !3892
  %65 = getelementptr inbounds nuw i8, ptr %_33.i.i, i64 88
  %config.val6.i.i.i.i.i = load i64, ptr %65, align 8, !alias.scope !107711, !noalias !107712, !noundef !3892
; invoke <alloc::vec::Vec<alloc::string::String> as core::clone::Clone>::clone
  invoke fastcc void @<alloc::vec::Vec<alloc::string::String> as core::clone::Clone>::clone(ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %_2.i.i.i.i.i, ptr nonnull %config.val.i.i.i.i.i, i64 %config.val6.i.i.i.i.i)
          to label %bb7.i.i.i.i.i unwind label %cleanup7.i.i.i.i, !noalias !107681

bb4.i43.i.i.i.i:                                  ; preds = %bb4.i.i49.i.i
  store i64 0, ptr %_2.i.i.i.i.i, align 8, !noalias !107709
  %66 = getelementptr inbounds nuw i8, ptr %_2.i.i.i.i.i, i64 8
  store ptr inttoptr (i64 8 to ptr), ptr %66, align 8, !noalias !107709
  %67 = getelementptr inbounds nuw i8, ptr %_2.i.i.i.i.i, i64 16
  store i64 0, ptr %67, align 8, !noalias !107709
  br label %bb7.i.i.i.i.i

bb7.i.i.i.i.i:                                    ; preds = %bb4.i43.i.i.i.i, %bb5.i.i.i.i.i
  %68 = getelementptr inbounds nuw i8, ptr %_33.i.i, i64 96
  %69 = load i64, ptr %68, align 8, !range !3909, !alias.scope !107711, !noalias !107712, !noundef !3892
  %.not4.i.i.i.i.i = icmp eq i64 %69, -1
  br i1 %.not4.i.i.i.i.i, label %bb5.i.i.i.i, label %bb9.i.i.i.i.i

bb9.i.i.i.i.i:                                    ; preds = %bb7.i.i.i.i.i
  call void @llvm.lifetime.start.p0(ptr nonnull %_12.i.i.i.i.i), !noalias !107709
  %70 = getelementptr inbounds nuw i8, ptr %_33.i.i, i64 104
  %.val.i.i.i.i.i = load ptr, ptr %70, align 8, !alias.scope !107711, !noalias !107712, !nonnull !3892, !noundef !3892
  %71 = getelementptr inbounds nuw i8, ptr %_33.i.i, i64 112
  %.val5.i.i.i.i.i = load i64, ptr %71, align 8, !alias.scope !107711, !noalias !107712, !noundef !3892
; invoke <alloc::vec::Vec<alloc::string::String> as core::clone::Clone>::clone
  invoke fastcc void @<alloc::vec::Vec<alloc::string::String> as core::clone::Clone>::clone(ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %_12.i.i.i.i.i, ptr nonnull %.val.i.i.i.i.i, i64 %.val5.i.i.i.i.i)
          to label %bb10.i.i.i.i.i unwind label %cleanup.i42.i.i.i.i, !noalias !107709

cleanup.i42.i.i.i.i:                              ; preds = %bb9.i.i.i.i.i
  %72 = landingpad { ptr, i32 }
          cleanup
; call core::ptr::drop_glue::<alloc::vec::Vec<alloc::string::String>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<alloc::string::String>>(ptr noalias nofree noundef align 8 dereferenceable(24) %_2.i.i.i.i.i) #79, !noalias !107709
  br label %bb19.i.i.i.i

bb10.i.i.i.i.i:                                   ; preds = %bb9.i.i.i.i.i
  %_5.sroa.0.0.copyload.i.i.i.i.i = load i64, ptr %_12.i.i.i.i.i, align 8, !noalias !107709
  %_5.sroa.4.0._12.sroa_idx.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %_12.i.i.i.i.i, i64 8
  %_5.sroa.4.0.copyload.i.i.i.i.i = load ptr, ptr %_5.sroa.4.0._12.sroa_idx.i.i.i.i.i, align 8, !noalias !107709
  %_5.sroa.5.0._12.sroa_idx.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %_12.i.i.i.i.i, i64 16
  %_5.sroa.5.0.copyload.i.i.i.i.i = load i64, ptr %_5.sroa.5.0._12.sroa_idx.i.i.i.i.i, align 8, !noalias !107709
  call void @llvm.lifetime.end.p0(ptr nonnull %_12.i.i.i.i.i), !noalias !107709
  br label %bb5.i.i.i.i

bb19.i.i.i.i:                                     ; preds = %bb33.i.i.i.i, %bb18.i.i.i.i, %bb34.thread135.i.i.i.i, %cleanup7.i.i.i.i, %cleanup.i42.i.i.i.i
  %_40.sroa.0.6.i.i.i.i = phi i8 [ 0, %bb33.i.i.i.i ], [ 0, %bb34.thread135.i.i.i.i ], [ 1, %cleanup.i42.i.i.i.i ], [ 1, %cleanup7.i.i.i.i ], [ 0, %bb18.i.i.i.i ]
  %.pn28.pn.i.i.i.i = phi { ptr, i32 } [ %lpad.thr_comm.split-lp134.i.i.i.i, %bb33.i.i.i.i ], [ %lpad.thr_comm133.i.i.i.i, %bb34.thread135.i.i.i.i ], [ %72, %cleanup.i42.i.i.i.i ], [ %75, %cleanup7.i.i.i.i ], [ %.pn.i.i.i.i, %bb18.i.i.i.i ]
  %73 = load ptr, ptr %aggregates.i.i.i.i, align 8, !alias.scope !107713, !noalias !107681, !noundef !3892
  %74 = icmp eq ptr %73, null
  br i1 %74, label %bb20.i.i.i.i, label %bb2.i.i.i.i.i

bb2.i.i.i.i.i:                                    ; preds = %bb19.i.i.i.i
; invoke core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, alloc::sync::Arc<dyn purrdf_sparql_eval::agg_fn::CustomAggregate>)>>
  invoke fastcc void @core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, alloc::sync::Arc<dyn purrdf_sparql_eval::agg_fn::CustomAggregate>)>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(40) %aggregates.i.i.i.i)
          to label %bb20.i.i.i.i unwind label %terminate.i.i.i.i, !noalias !107681

cleanup7.i.i.i.i:                                 ; preds = %bb5.i.i.i.i.i
  %75 = landingpad { ptr, i32 }
          cleanup
  br label %bb19.i.i.i.i

bb5.i.i.i.i:                                      ; preds = %bb10.i.i.i.i.i, %bb7.i.i.i.i.i
  %_4.sroa.6.0.i.i.i.i.i = phi i64 [ %_5.sroa.5.0.copyload.i.i.i.i.i, %bb10.i.i.i.i.i ], [ 0, %bb7.i.i.i.i.i ]
  %_4.sroa.5.0.i.i.i.i.i = phi ptr [ %_5.sroa.4.0.copyload.i.i.i.i.i, %bb10.i.i.i.i.i ], [ inttoptr (i64 8 to ptr), %bb7.i.i.i.i.i ]
  %_4.sroa.0.0.i.i.i.i.i = phi i64 [ %_5.sroa.0.0.copyload.i.i.i.i.i, %bb10.i.i.i.i.i ], [ 0, %bb7.i.i.i.i.i ]
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %parser_options.i.i.i.i, ptr noundef nonnull align 8 dereferenceable(24) %_2.i.i.i.i.i, i64 24, i1 false), !noalias !107681
  %76 = getelementptr inbounds nuw i8, ptr %parser_options.i.i.i.i, i64 24
  store i64 %_4.sroa.0.0.i.i.i.i.i, ptr %76, align 8, !noalias !107681
  %_4.sroa.5.0..sroa_idx.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %parser_options.i.i.i.i, i64 32
  store ptr %_4.sroa.5.0.i.i.i.i.i, ptr %_4.sroa.5.0..sroa_idx.i.i.i.i.i, align 8, !noalias !107681
  %_4.sroa.6.0..sroa_idx.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %parser_options.i.i.i.i, i64 40
  store i64 %_4.sroa.6.0.i.i.i.i.i, ptr %_4.sroa.6.0..sroa_idx.i.i.i.i.i, align 8, !noalias !107681
  %77 = getelementptr inbounds nuw i8, ptr %parser_options.i.i.i.i, i64 48
  store i64 0, ptr %77, align 8, !noalias !107681
  %_6.sroa.4.0..sroa_idx.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %parser_options.i.i.i.i, i64 56
  store ptr inttoptr (i64 8 to ptr), ptr %_6.sroa.4.0..sroa_idx.i.i.i.i.i, align 8, !noalias !107681
  %_6.sroa.5.0..sroa_idx.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %parser_options.i.i.i.i, i64 64
  store i64 0, ptr %_6.sroa.5.0..sroa_idx.i.i.i.i.i, align 8, !noalias !107681
  call void @llvm.lifetime.end.p0(ptr nonnull %_2.i.i.i.i.i), !noalias !107709
  call void @llvm.lifetime.start.p0(ptr nonnull %engine.i.i.i.i), !noalias !107681
; invoke purrdf_native::py_store::query::build_engine
  invoke fastcc void @purrdf_native::py_store::query::build_engine(ptr noalias nofree noundef align 8 captures(address) dereferenceable(520) %engine.i.i.i.i, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(168) %38)
          to label %bb6.i.i.i.i unwind label %bb33.i.i.i.i, !noalias !107695

bb34.thread135.i.i.i.i:                           ; preds = %bb8.i.i.i.i, %bb55.i.i.i.i
  %lpad.thr_comm133.i.i.i.i = landingpad { ptr, i32 }
          cleanup
  br label %bb19.i.i.i.i

bb6.i.i.i.i:                                      ; preds = %bb5.i.i.i.i
  call void @llvm.lifetime.start.p0(ptr nonnull %_21.sroa.6.i.i.sroa.6.i.i)
  call void @llvm.lifetime.start.p0(ptr nonnull %_21.sroa.7.i.i.i.i)
  %_72.i.i.i.i = load ptr, ptr %dataset.i.i.i.i, align 8, !noalias !107681, !nonnull !3892, !noundef !3892
  %_23.i.i.i.i = getelementptr inbounds nuw i8, ptr %_72.i.i.i.i, i64 16
  %78 = getelementptr inbounds nuw i8, ptr %_33.i.i, i64 32
  %_76.i.i.i.i = load ptr, ptr %78, align 8, !alias.scope !107705, !noalias !107695, !nonnull !3892, !noundef !3892
  %79 = getelementptr inbounds nuw i8, ptr %_33.i.i, i64 40
  %_75.i.i.i.i = load i64, ptr %79, align 8, !alias.scope !107705, !noalias !107695, !noundef !3892
  call void @llvm.lifetime.start.p0(ptr nonnull %_29.i.i.i.i), !noalias !107681
  call void @llvm.lifetime.start.p0(ptr nonnull %_30.sroa.6.i.i.sroa.0.i.i)
  call void @llvm.lifetime.start.p0(ptr nonnull %_31.i.i.i.i), !noalias !107681
  %80 = load ptr, ptr %registry.i.i.i.i, align 8, !noalias !107681, !noundef !3892
  %.not24.i.i.i.i = icmp eq ptr %80, null
  %.registry.i.i.i.i = select i1 %.not24.i.i.i.i, ptr null, ptr %registry.i.i.i.i
  %81 = load ptr, ptr %aggregates.i.i.i.i, align 8, !noalias !107681, !noundef !3892
  %.not25.i.i.i.i = icmp eq ptr %81, null
  %_34.sroa.0.0.i.i.i.i = select i1 %.not25.i.i.i.i, ptr null, ptr %aggregates.i.i.i.i
; invoke purrdf_native::py_store::env::extension_env
  invoke fastcc void @purrdf_native::py_store::env::extension_env(ptr noalias nofree noundef align 8 captures(none) dereferenceable(352) %_31.i.i.i.i, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(72) %parser_options.i.i.i.i, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable_or_null(72) %.registry.i.i.i.i, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable_or_null(40) %_34.sroa.0.0.i.i.i.i)
          to label %bb7.i.i.i.i unwind label %cleanup9.i.i.i.i, !noalias !107681

bb18.i.i.i.i:                                     ; preds = %cleanup10.body.i.i.i.i, %cleanup9.i.i.i.i
  %.pn.i.i.i.i = phi { ptr, i32 } [ %82, %cleanup9.i.i.i.i ], [ %eh.lpad-body48.i.i.i.i, %cleanup10.body.i.i.i.i ]
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::engine::NativeSparqlEngine>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::engine::NativeSparqlEngine>(ptr noalias nofree noundef align 8 dereferenceable(520) %engine.i.i.i.i) #79
          to label %bb19.i.i.i.i unwind label %terminate.i.i.i.i, !noalias !107681

cleanup9.i.i.i.i:                                 ; preds = %bb58.i.i.i.i, %bb6.i.i.i.i
  %82 = landingpad { ptr, i32 }
          cleanup
  br label %bb18.i.i.i.i

bb7.i.i.i.i:                                      ; preds = %bb6.i.i.i.i
  %83 = load i64, ptr %_31.i.i.i.i, align 8, !range !3909, !noalias !107681, !noundef !3892
  %84 = icmp eq i64 %83, -1
  %85 = getelementptr inbounds nuw i8, ptr %_31.i.i.i.i, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %_30.sroa.6.i.i.sroa.0.i.i, ptr noundef nonnull align 8 dereferenceable(16) %85, i64 16, i1 false), !noalias !107681
  %_30.sroa.6.i.i.sroa.6.0..sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %_31.i.i.i.i, i64 24
  %_30.sroa.6.i.i.sroa.6.0.copyload.i.i = load i64, ptr %_30.sroa.6.i.i.sroa.6.0..sroa_idx.i.i, align 8, !noalias !107681
  %_30.sroa.6.i.i.sroa.7.0..sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %_31.i.i.i.i, i64 32
  %_30.sroa.6.i.i.sroa.7.0.copyload.i.i = load ptr, ptr %_30.sroa.6.i.i.sroa.7.0..sroa_idx.i.i, align 8, !noalias !107681
  %_30.sroa.6.i.i.sroa.8.0..sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %_31.i.i.i.i, i64 40
  %_30.sroa.6.i.i.sroa.8.0.copyload.i.i = load ptr, ptr %_30.sroa.6.i.i.sroa.8.0..sroa_idx.i.i, align 8, !noalias !107681
  %_30.sroa.6.i.i.sroa.9.0..sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %_31.i.i.i.i, i64 48
  %_30.sroa.6.i.i.sroa.9.0.copyload.i.i = load i32, ptr %_30.sroa.6.i.i.sroa.9.0..sroa_idx.i.i, align 8, !noalias !107681
  %_30.sroa.6.i.i.sroa.10.0..sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %_31.i.i.i.i, i64 52
  %_30.sroa.6.i.i.sroa.10.0.copyload.i.i = load i32, ptr %_30.sroa.6.i.i.sroa.10.0..sroa_idx.i.i, align 4, !noalias !107681
  br i1 %84, label %bb55.i.i.i.i, label %bb56.i.i.i.i

bb55.i.i.i.i:                                     ; preds = %bb7.i.i.i.i
  call void @llvm.lifetime.end.p0(ptr nonnull %_31.i.i.i.i), !noalias !107681
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %_32.sroa.10.i.i, ptr noundef nonnull align 8 dereferenceable(16) %_30.sroa.6.i.i.sroa.0.i.i, i64 16, i1 false), !noalias !107704
  call void @llvm.lifetime.end.p0(ptr nonnull %_30.sroa.6.i.i.sroa.0.i.i)
  call void @llvm.lifetime.end.p0(ptr nonnull %_29.i.i.i.i), !noalias !107681
  call void @llvm.lifetime.end.p0(ptr nonnull %_21.sroa.6.i.i.sroa.6.i.i)
  call void @llvm.lifetime.end.p0(ptr nonnull %_21.sroa.7.i.i.i.i)
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::engine::NativeSparqlEngine>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::engine::NativeSparqlEngine>(ptr noalias nofree noundef align 8 dereferenceable(520) %engine.i.i.i.i)
          to label %bb13.i.i.i.i unwind label %bb34.thread135.i.i.i.i, !noalias !107681

bb56.i.i.i.i:                                     ; preds = %bb7.i.i.i.i
  %_82.sroa.5.0._31.sroa_idx.i.i.i.i = getelementptr inbounds nuw i8, ptr %_31.i.i.i.i, i64 56
  %val3.sroa.5.0._29.sroa_idx.i.i.i.i = getelementptr inbounds nuw i8, ptr %_29.i.i.i.i, i64 56
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(296) %val3.sroa.5.0._29.sroa_idx.i.i.i.i, ptr noundef nonnull align 8 dereferenceable(296) %_82.sroa.5.0._31.sroa_idx.i.i.i.i, i64 296, i1 false), !noalias !107681
  call void @llvm.lifetime.end.p0(ptr nonnull %_31.i.i.i.i), !noalias !107681
  %val3.sroa.4.0._29.sroa_idx.i.i.i.i = getelementptr inbounds nuw i8, ptr %_29.i.i.i.i, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %val3.sroa.4.0._29.sroa_idx.i.i.i.i, ptr noundef nonnull align 8 dereferenceable(16) %_30.sroa.6.i.i.sroa.0.i.i, i64 16, i1 false), !noalias !107681
  %_30.sroa.6.i.i.sroa.6.0.val3.sroa.4.0._29.sroa_idx.i.i.sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %_29.i.i.i.i, i64 24
  store i64 %_30.sroa.6.i.i.sroa.6.0.copyload.i.i, ptr %_30.sroa.6.i.i.sroa.6.0.val3.sroa.4.0._29.sroa_idx.i.i.sroa_idx.i.i, align 8, !noalias !107681
  %_30.sroa.6.i.i.sroa.7.0.val3.sroa.4.0._29.sroa_idx.i.i.sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %_29.i.i.i.i, i64 32
  store ptr %_30.sroa.6.i.i.sroa.7.0.copyload.i.i, ptr %_30.sroa.6.i.i.sroa.7.0.val3.sroa.4.0._29.sroa_idx.i.i.sroa_idx.i.i, align 8, !noalias !107681
  %_30.sroa.6.i.i.sroa.8.0.val3.sroa.4.0._29.sroa_idx.i.i.sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %_29.i.i.i.i, i64 40
  store ptr %_30.sroa.6.i.i.sroa.8.0.copyload.i.i, ptr %_30.sroa.6.i.i.sroa.8.0.val3.sroa.4.0._29.sroa_idx.i.i.sroa_idx.i.i, align 8, !noalias !107681
  %_30.sroa.6.i.i.sroa.9.0.val3.sroa.4.0._29.sroa_idx.i.i.sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %_29.i.i.i.i, i64 48
  store i32 %_30.sroa.6.i.i.sroa.9.0.copyload.i.i, ptr %_30.sroa.6.i.i.sroa.9.0.val3.sroa.4.0._29.sroa_idx.i.i.sroa_idx.i.i, align 8, !noalias !107681
  %_30.sroa.6.i.i.sroa.10.0.val3.sroa.4.0._29.sroa_idx.i.i.sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %_29.i.i.i.i, i64 52
  store i32 %_30.sroa.6.i.i.sroa.10.0.copyload.i.i, ptr %_30.sroa.6.i.i.sroa.10.0.val3.sroa.4.0._29.sroa_idx.i.i.sroa_idx.i.i, align 4, !noalias !107681
  store i64 %83, ptr %_29.i.i.i.i, align 8, !noalias !107681
  call void @llvm.lifetime.start.p0(ptr nonnull %result.i.i.i.i.i.i), !noalias !107716
  call void @llvm.experimental.noalias.scope.decl(metadata !107723)
  call void @llvm.experimental.noalias.scope.decl(metadata !107726)
  call void @llvm.lifetime.start.p0(ptr nonnull %admitted.i.i.i.i.i.i.i.i), !noalias !107729
; invoke <purrdf_sparql_eval::engine::AdmittedSubstitutions>::requested
  invoke void @<purrdf_sparql_eval::engine::AdmittedSubstitutions>::requested(ptr noalias nofree noundef nonnull sret([288 x i8]) align 8 captures(none) dereferenceable(288) %admitted.i.i.i.i.i.i.i.i, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) %_76.i.i.i.i, i64 noundef %_75.i.i.i.i, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) inttoptr (i64 8 to ptr), i64 noundef 0)
          to label %.noexc47.i.i.i.i unwind label %cleanup10.i.i.i.i, !noalias !107681

.noexc47.i.i.i.i:                                 ; preds = %bb56.i.i.i.i
  call void @llvm.lifetime.start.p0(ptr nonnull %prepared.i.i.i.i.i.i.i.i), !noalias !107729
  call void @llvm.lifetime.start.p0(ptr nonnull %_5.i.i.i.i.i.i.i.i), !noalias !107729
  %_7.i.i.i.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %admitted.i.i.i.i.i.i.i.i, i64 16
; invoke <purrdf_sparql_eval::engine::NativeSparqlEngine>::prepare_request
  invoke void @<purrdf_sparql_eval::engine::NativeSparqlEngine>::prepare_request(ptr noalias nofree noundef nonnull sret([96 x i8]) align 8 captures(address) dereferenceable(96) %_5.i.i.i.i.i.i.i.i, ptr noundef nonnull align 8 %engine.i.i.i.i, ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) %query.0, i64 noundef %query.1, ptr noalias nofree noundef readonly captures(address, read_provenance) null, i64 undef, ptr noundef nonnull align 8 %_29.i.i.i.i, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(272) %_7.i.i.i.i.i.i.i.i)
          to label %bb2.i.i.i.i.i.i.i.i unwind label %cleanup.i.i.i.i.i.i.i.i, !noalias !107729

bb14.i.i.i.i.i.i.i.i:                             ; preds = %cleanup3.i.i.i.i.i.i.i.i, %bb2.i.i.i.i.i.i.i.i.i.i, %cleanup.i.i.i.i.i.i.i.i
  %.pn9.i.i.i.i.i.i.i.i = phi { ptr, i32 } [ %86, %cleanup.i.i.i.i.i.i.i.i ], [ %89, %bb2.i.i.i.i.i.i.i.i.i.i ], [ %89, %cleanup3.i.i.i.i.i.i.i.i ]
; call core::ptr::drop_glue::<purrdf_sparql_eval::engine::AdmittedSubstitutions>
  call fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::engine::AdmittedSubstitutions>(ptr noalias nofree noundef align 8 dereferenceable(288) %admitted.i.i.i.i.i.i.i.i) #79, !noalias !107729
  br label %cleanup10.body.i.i.i.i

cleanup.i.i.i.i.i.i.i.i:                          ; preds = %bb2.i.i18.i.i.i.i.i.i.i.i, %.noexc47.i.i.i.i
  %86 = landingpad { ptr, i32 }
          cleanup
  br label %bb14.i.i.i.i.i.i.i.i

bb2.i.i.i.i.i.i.i.i:                              ; preds = %.noexc47.i.i.i.i
  %87 = load i64, ptr %_5.i.i.i.i.i.i.i.i, align 8, !range !3909, !noalias !107729, !noundef !3892
  %.not.i.i.i.i.i.i.i.i = icmp eq i64 %87, -1
  %88 = getelementptr inbounds nuw i8, ptr %_5.i.i.i.i.i.i.i.i, i64 8
  %_31.i.i.i.i.i.i.i.i = load ptr, ptr %88, align 8, !noalias !107729
  br i1 %.not.i.i.i.i.i.i.i.i, label %bb19.i.i.i.i.i.i.i.i, label %bb9.i.i.i.i.i.i.i.i

bb2.i.i.i.i.i.i.i.i.i.i:                          ; preds = %cleanup3.i.i.i.i.i.i.i.i
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::engine::PreparedQuery>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::engine::PreparedQuery>>::drop_slow(ptr noalias nofree noundef nonnull align 8 dereferenceable(8) %prepared.i.i.i.i.i.i.i.i) #81
          to label %bb14.i.i.i.i.i.i.i.i unwind label %terminate.i.i.i.i.i.i.i.i, !noalias !107729

bb19.i.i.i.i.i.i.i.i:                             ; preds = %bb2.i.i.i.i.i.i.i.i
  call void @llvm.lifetime.end.p0(ptr nonnull %_5.i.i.i.i.i.i.i.i), !noalias !107729
  store ptr %_31.i.i.i.i.i.i.i.i, ptr %prepared.i.i.i.i.i.i.i.i, align 8, !noalias !107729
  %_19.i.i.i.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %_31.i.i.i.i.i.i.i.i, i64 16
  call void @llvm.lifetime.start.p0(ptr nonnull %_20.i.i.i.i.i.i.i.i), !noalias !107729
  store i64 %_52.sroa.0.0.copyload.i.i, ptr %_20.i.i.i.i.i.i.i.i, align 8, !noalias !107729
  %_26.sroa.5.0._20.i.i.i.i.sroa_idx.i.i.i.i = getelementptr inbounds nuw i8, ptr %_20.i.i.i.i.i.i.i.i, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(80) %_26.sroa.5.0._20.i.i.i.i.sroa_idx.i.i.i.i, ptr noundef nonnull align 8 dereferenceable(80) getelementptr inbounds nuw (i8, ptr @anon.5ac75c6ab29af51a474fa1a77778ee04.4, i64 8), i64 80, i1 false), !noalias !107729
  %_26.sroa.6.0._20.i.i.i.i.sroa_idx.i.i.i.i = getelementptr inbounds nuw i8, ptr %_20.i.i.i.i.i.i.i.i, i64 88
  store ptr %_29.i.i.i.i, ptr %_26.sroa.6.0._20.i.i.i.i.sroa_idx.i.i.i.i, align 8, !noalias !107729
  %_26.sroa.8.0._20.i.i.i.i.sroa_idx.i.i.i.i = getelementptr inbounds nuw i8, ptr %_20.i.i.i.i.i.i.i.i, i64 96
  store ptr inttoptr (i64 8 to ptr), ptr %_26.sroa.8.0._20.i.i.i.i.sroa_idx.i.i.i.i, align 8, !noalias !107729
  %_26.sroa.10.0._20.i.i.i.i.sroa_idx.i.i.i.i = getelementptr inbounds nuw i8, ptr %_20.i.i.i.i.i.i.i.i, i64 104
  store i64 0, ptr %_26.sroa.10.0._20.i.i.i.i.sroa_idx.i.i.i.i, align 8, !noalias !107729
  %_26.sroa.11.0._20.i.i.i.i.sroa_idx.i.i.i.i = getelementptr inbounds nuw i8, ptr %_20.i.i.i.i.i.i.i.i, i64 112
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(64) %_26.sroa.11.0._20.i.i.i.i.sroa_idx.i.i.i.i, ptr noundef nonnull align 8 dereferenceable(64) getelementptr inbounds nuw (i8, ptr @anon.5ac75c6ab29af51a474fa1a77778ee04.4, i64 112), i64 64, i1 false), !noalias !107729
; invoke <purrdf_sparql_eval::engine::NativeSparqlEngine>::query_prepared_admitted::<purrdf_core::ir::dataset::RdfDataset, purrdf_core::dataset_view::NoopReservation<!>>
  invoke fastcc void @<purrdf_sparql_eval::engine::NativeSparqlEngine>::query_prepared_admitted::<purrdf_core::ir::dataset::RdfDataset, purrdf_core::dataset_view::NoopReservation<!>>(ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(96) %result.i.i.i.i.i.i, ptr noundef nonnull align 8 %engine.i.i.i.i, ptr noundef nonnull align 8 %_23.i.i.i.i, ptr noundef nonnull align 8 %_19.i.i.i.i.i.i.i.i, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) %_76.i.i.i.i, i64 noundef %_75.i.i.i.i, ptr noalias nofree noundef align 8 captures(address) dereferenceable(176) %_20.i.i.i.i.i.i.i.i)
          to label %bb5.i.i.i.i.i.i.i.i unwind label %cleanup3.i.i.i.i.i.i.i.i

cleanup3.i.i.i.i.i.i.i.i:                         ; preds = %bb19.i.i.i.i.i.i.i.i
  %89 = landingpad { ptr, i32 }
          cleanup
  %_2.i.i.i.i.i.i.i.i.i.i = atomicrmw sub ptr %_31.i.i.i.i.i.i.i.i, i64 1 release, align 8, !noalias !107732
  %90 = icmp eq i64 %_2.i.i.i.i.i.i.i.i.i.i, 1
  br i1 %90, label %bb2.i.i.i.i.i.i.i.i.i.i, label %bb14.i.i.i.i.i.i.i.i

bb5.i.i.i.i.i.i.i.i:                              ; preds = %bb19.i.i.i.i.i.i.i.i
  call void @llvm.lifetime.end.p0(ptr nonnull %_20.i.i.i.i.i.i.i.i), !noalias !107729
  %_2.i.i17.i.i.i.i.i.i.i.i = atomicrmw sub ptr %_31.i.i.i.i.i.i.i.i, i64 1 release, align 8, !noalias !107737
  %91 = icmp eq i64 %_2.i.i17.i.i.i.i.i.i.i.i, 1
  br i1 %91, label %bb2.i.i18.i.i.i.i.i.i.i.i, label %bb7.i.i.i.i.i.i.i.i

bb2.i.i18.i.i.i.i.i.i.i.i:                        ; preds = %bb5.i.i.i.i.i.i.i.i
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::engine::PreparedQuery>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::engine::PreparedQuery>>::drop_slow(ptr noalias nofree noundef nonnull align 8 dereferenceable(8) %prepared.i.i.i.i.i.i.i.i) #81
          to label %bb7.i.i.i.i.i.i.i.i unwind label %cleanup.i.i.i.i.i.i.i.i, !noalias !107729

bb7.i.i.i.i.i.i.i.i:                              ; preds = %bb2.i.i18.i.i.i.i.i.i.i.i, %bb5.i.i.i.i.i.i.i.i
  call void @llvm.lifetime.end.p0(ptr nonnull %prepared.i.i.i.i.i.i.i.i), !noalias !107729
  call void @llvm.experimental.noalias.scope.decl(metadata !107742)
  call void @llvm.experimental.noalias.scope.decl(metadata !107745)
  %_1.val.i.i.i.i.i.i.i.i.i.i = load i64, ptr %_7.i.i.i.i.i.i.i.i, align 8, !range !10771, !alias.scope !107748, !noalias !107729, !noundef !3892
  %92 = icmp ugt i64 %_1.val.i.i.i.i.i.i.i.i.i.i, 9
  br i1 %92, label %core::ptr::drop_glue::<alloc::vec::Vec<&str>> (.exit.i.i.i.i.i.i.i.i.i.i.i.i), label %bb4.i.i.i.i.i.i.i.i.i.i

core::ptr::drop_glue::<alloc::vec::Vec<&str>> (.exit.i.i.i.i.i.i.i.i.i.i.i.i): ; preds = %bb7.i.i.i.i.i.i.i.i
  %93 = getelementptr inbounds nuw i8, ptr %admitted.i.i.i.i.i.i.i.i, i64 24
  %_1.val4.i.i.i.i.i.i.i.i.i.i = load ptr, ptr %93, align 8, !alias.scope !107751, !noalias !107729, !nonnull !3892, !noundef !3892
  %94 = shl i64 %_1.val.i.i.i.i.i.i.i.i.i.i, 4
  %alloc_size.i.i.i.i5.i.i.i.i.i.i.i.i.i.i.i.i.i = add i64 %94, -16
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %_1.val4.i.i.i.i.i.i.i.i.i.i, i64 noundef %alloc_size.i.i.i.i5.i.i.i.i.i.i.i.i.i.i.i.i.i, i64 noundef range(i64 1, -9223372036854775807) 8) #77, !noalias !107752
  br label %bb4.i.i.i.i.i.i.i.i.i.i

bb4.i.i.i.i.i.i.i.i.i.i:                          ; preds = %core::ptr::drop_glue::<alloc::vec::Vec<&str>> (.exit.i.i.i.i.i.i.i.i.i.i.i.i), %bb7.i.i.i.i.i.i.i.i
  %95 = getelementptr inbounds nuw i8, ptr %admitted.i.i.i.i.i.i.i.i, i64 152
  %.val2.i.i.i.i.i.i.i.i.i.i = load i64, ptr %95, align 8, !range !10771, !alias.scope !107748, !noalias !107729, !noundef !3892
  %96 = icmp ugt i64 %.val2.i.i.i.i.i.i.i.i.i.i, 9
  br i1 %96, label %bb11.sink.split.i.i.i.i.i.i.i.i, label %bb57.i.i.i.i

bb11.sink.split.i.i.i.i.i.i.i.i:                  ; preds = %bb4.i.i22.i.i.i.i.i.i.i.i, %bb4.i.i.i.i.i.i.i.i.i.i
  %.val2.i.i.sink.i.i.i.i.i.i.i.i = phi i64 [ %.val2.i.i23.i.i.i.i.i.i.i.i, %bb4.i.i22.i.i.i.i.i.i.i.i ], [ %.val2.i.i.i.i.i.i.i.i.i.i, %bb4.i.i.i.i.i.i.i.i.i.i ]
  %97 = getelementptr inbounds nuw i8, ptr %admitted.i.i.i.i.i.i.i.i, i64 160
  %.val3.i.i.i.i.i.i.i.i.i.i = load ptr, ptr %97, align 8, !noalias !107729, !nonnull !3892, !noundef !3892
  %98 = shl i64 %.val2.i.i.sink.i.i.i.i.i.i.i.i, 4
  %alloc_size.i.i.i.i5.i.i.i9.i.i.i.i.i.i.i.i.i.i = add i64 %98, -16
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %.val3.i.i.i.i.i.i.i.i.i.i, i64 noundef %alloc_size.i.i.i.i5.i.i.i9.i.i.i.i.i.i.i.i.i.i, i64 noundef range(i64 1, -9223372036854775807) 8) #77, !noalias !107729
  br label %bb57.i.i.i.i

bb9.i.i.i.i.i.i.i.i:                              ; preds = %bb2.i.i.i.i.i.i.i.i
  %_32.sroa.5.0._5.sroa_idx.i.i.i.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %_5.i.i.i.i.i.i.i.i, i64 16
  %_37.sroa.5.0._0.sroa_idx.i.i.i.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %result.i.i.i.i.i.i, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(80) %_37.sroa.5.0._0.sroa_idx.i.i.i.i.i.i.i.i, ptr noundef nonnull align 8 dereferenceable(80) %_32.sroa.5.0._5.sroa_idx.i.i.i.i.i.i.i.i, i64 80, i1 false), !noalias !107755
  call void @llvm.lifetime.end.p0(ptr nonnull %_5.i.i.i.i.i.i.i.i), !noalias !107729
  store i64 %87, ptr %result.i.i.i.i.i.i, align 8, !alias.scope !107756, !noalias !107755
  %_37.sroa.4.0._0.sroa_idx.i.i.i.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %result.i.i.i.i.i.i, i64 8
  store ptr %_31.i.i.i.i.i.i.i.i, ptr %_37.sroa.4.0._0.sroa_idx.i.i.i.i.i.i.i.i, align 8, !alias.scope !107756, !noalias !107755
  call void @llvm.lifetime.end.p0(ptr nonnull %prepared.i.i.i.i.i.i.i.i), !noalias !107729
  call void @llvm.experimental.noalias.scope.decl(metadata !107757)
  call void @llvm.experimental.noalias.scope.decl(metadata !107760)
  %_1.val.i.i21.i.i.i.i.i.i.i.i = load i64, ptr %_7.i.i.i.i.i.i.i.i, align 8, !range !10771, !alias.scope !107763, !noalias !107729, !noundef !3892
  %99 = icmp ugt i64 %_1.val.i.i21.i.i.i.i.i.i.i.i, 9
  br i1 %99, label %core::ptr::drop_glue::<alloc::vec::Vec<&str>> (.exit.i.i.i.i27.i.i.i.i.i.i.i.i), label %bb4.i.i22.i.i.i.i.i.i.i.i

core::ptr::drop_glue::<alloc::vec::Vec<&str>> (.exit.i.i.i.i27.i.i.i.i.i.i.i.i): ; preds = %bb9.i.i.i.i.i.i.i.i
  %100 = getelementptr inbounds nuw i8, ptr %admitted.i.i.i.i.i.i.i.i, i64 24
  %_1.val4.i.i28.i.i.i.i.i.i.i.i = load ptr, ptr %100, align 8, !alias.scope !107766, !noalias !107729, !nonnull !3892, !noundef !3892
  %101 = shl i64 %_1.val.i.i21.i.i.i.i.i.i.i.i, 4
  %alloc_size.i.i.i.i5.i.i.i.i.i29.i.i.i.i.i.i.i.i = add i64 %101, -16
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %_1.val4.i.i28.i.i.i.i.i.i.i.i, i64 noundef %alloc_size.i.i.i.i5.i.i.i.i.i29.i.i.i.i.i.i.i.i, i64 noundef range(i64 1, -9223372036854775807) 8) #77, !noalias !107767
  br label %bb4.i.i22.i.i.i.i.i.i.i.i

bb4.i.i22.i.i.i.i.i.i.i.i:                        ; preds = %core::ptr::drop_glue::<alloc::vec::Vec<&str>> (.exit.i.i.i.i27.i.i.i.i.i.i.i.i), %bb9.i.i.i.i.i.i.i.i
  %102 = getelementptr inbounds nuw i8, ptr %admitted.i.i.i.i.i.i.i.i, i64 152
  %.val2.i.i23.i.i.i.i.i.i.i.i = load i64, ptr %102, align 8, !range !10771, !alias.scope !107763, !noalias !107729, !noundef !3892
  %103 = icmp ugt i64 %.val2.i.i23.i.i.i.i.i.i.i.i, 9
  br i1 %103, label %bb11.sink.split.i.i.i.i.i.i.i.i, label %bb57.i.i.i.i

terminate.i.i.i.i.i.i.i.i:                        ; preds = %bb2.i.i.i.i.i.i.i.i.i.i
  %104 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #75, !noalias !107729
  unreachable

cleanup10.i.i.i.i:                                ; preds = %bb1.i.i.i.i.i, %bb56.i.i.i.i
  %105 = landingpad { ptr, i32 }
          cleanup
  br label %cleanup10.body.i.i.i.i

cleanup10.body.i.i.i.i:                           ; preds = %cleanup.body.i53.i.i.i.i, %cleanup10.i.i.i.i, %bb14.i.i.i.i.i.i.i.i
  %eh.lpad-body48.i.i.i.i = phi { ptr, i32 } [ %.pn9.i.i.i.i.i.i.i.i, %bb14.i.i.i.i.i.i.i.i ], [ %105, %cleanup10.i.i.i.i ], [ %eh.lpad-body.i54.i.i.i.i, %cleanup.body.i53.i.i.i.i ]
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::extension_env::ExtensionEnv>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::extension_env::ExtensionEnv>(ptr noalias nofree noundef align 8 dereferenceable(352) %_29.i.i.i.i) #79
          to label %bb18.i.i.i.i unwind label %terminate.i.i.i.i, !noalias !107681

bb57.i.i.i.i:                                     ; preds = %bb4.i.i22.i.i.i.i.i.i.i.i, %bb11.sink.split.i.i.i.i.i.i.i.i, %bb4.i.i.i.i.i.i.i.i.i.i
  call void @llvm.lifetime.end.p0(ptr nonnull %admitted.i.i.i.i.i.i.i.i), !noalias !107729
  %_21.sroa.0.0.copyload115.i.i.i.i = load i64, ptr %result.i.i.i.i.i.i, align 8, !noalias !107770
  %_21.sroa.6.0.result.i.i.sroa_idx.i.i.i.i = getelementptr inbounds nuw i8, ptr %result.i.i.i.i.i.i, i64 8
  %_21.sroa.6.i.i.sroa.0.0.copyload.i.i = load i64, ptr %_21.sroa.6.0.result.i.i.sroa_idx.i.i.i.i, align 8, !noalias !107770
  %_21.sroa.6.i.i.sroa.6.0._21.sroa.6.0.result.i.i.sroa_idx.i.i.sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %result.i.i.i.i.i.i, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %_21.sroa.6.i.i.sroa.6.i.i, ptr noundef nonnull align 8 dereferenceable(16) %_21.sroa.6.i.i.sroa.6.0._21.sroa.6.0.result.i.i.sroa_idx.i.i.sroa_idx.i.i, i64 16, i1 false), !noalias !107770
  %_21.sroa.6.i.i.sroa.7.0._21.sroa.6.0.result.i.i.sroa_idx.i.i.sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %result.i.i.i.i.i.i, i64 32
  %_21.sroa.6.i.i.sroa.7.0.copyload.i.i = load i64, ptr %_21.sroa.6.i.i.sroa.7.0._21.sroa.6.0.result.i.i.sroa_idx.i.i.sroa_idx.i.i, align 8, !noalias !107770
  %_21.sroa.6.i.i.sroa.8.0._21.sroa.6.0.result.i.i.sroa_idx.i.i.sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %result.i.i.i.i.i.i, i64 40
  %_21.sroa.6.i.i.sroa.8.0.copyload.i.i = load ptr, ptr %_21.sroa.6.i.i.sroa.8.0._21.sroa.6.0.result.i.i.sroa_idx.i.i.sroa_idx.i.i, align 8, !noalias !107770
  %_21.sroa.6.i.i.sroa.9.0._21.sroa.6.0.result.i.i.sroa_idx.i.i.sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %result.i.i.i.i.i.i, i64 48
  %_21.sroa.6.i.i.sroa.9.0.copyload.i.i = load ptr, ptr %_21.sroa.6.i.i.sroa.9.0._21.sroa.6.0.result.i.i.sroa_idx.i.i.sroa_idx.i.i, align 8, !noalias !107770
  %_21.sroa.6.i.i.sroa.10.0._21.sroa.6.0.result.i.i.sroa_idx.i.i.sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %result.i.i.i.i.i.i, i64 56
  %_21.sroa.6.i.i.sroa.10.0.copyload.i.i = load i32, ptr %_21.sroa.6.i.i.sroa.10.0._21.sroa.6.0.result.i.i.sroa_idx.i.i.sroa_idx.i.i, align 8, !noalias !107770
  %_21.sroa.6.i.i.sroa.11.0._21.sroa.6.0.result.i.i.sroa_idx.i.i.sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %result.i.i.i.i.i.i, i64 60
  %_21.sroa.6.i.i.sroa.11.0.copyload.i.i = load i32, ptr %_21.sroa.6.i.i.sroa.11.0._21.sroa.6.0.result.i.i.sroa_idx.i.i.sroa_idx.i.i, align 4, !noalias !107770
  %_21.sroa.7.0.result.i.i.sroa_idx.i.i.i.i = getelementptr inbounds nuw i8, ptr %result.i.i.i.i.i.i, i64 64
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %_21.sroa.7.i.i.i.i, ptr noundef nonnull align 8 dereferenceable(32) %_21.sroa.7.0.result.i.i.sroa_idx.i.i.i.i, i64 32, i1 false), !noalias !107770
  call void @llvm.lifetime.end.p0(ptr nonnull %result.i.i.i.i.i.i), !noalias !107716
  %.not26.i.i.i.i = icmp eq i64 %_21.sroa.0.0.copyload115.i.i.i.i, -1
  br i1 %.not26.i.i.i.i, label %bb60.i.i.i.i, label %bb59.i.i.i.i

bb59.i.i.i.i:                                     ; preds = %bb57.i.i.i.i
  %_93.sroa.4.0._95.sroa_idx.i.i.i.i = getelementptr inbounds nuw i8, ptr %_95.i.i.i.i, i64 8
  call void @llvm.lifetime.start.p0(ptr nonnull %_95.i.i.i.i), !noalias !107681
  store i64 %_21.sroa.6.i.i.sroa.0.0.copyload.i.i, ptr %_93.sroa.4.0._95.sroa_idx.i.i.i.i, align 8, !noalias !107681
  %_21.sroa.6.i.i.sroa.6.0._93.sroa.4.0._95.sroa_idx.i.i.sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %_95.i.i.i.i, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %_21.sroa.6.i.i.sroa.6.0._93.sroa.4.0._95.sroa_idx.i.i.sroa_idx.i.i, ptr noundef nonnull align 8 dereferenceable(16) %_21.sroa.6.i.i.sroa.6.i.i, i64 16, i1 false), !noalias !107681
  %_21.sroa.6.i.i.sroa.7.0._93.sroa.4.0._95.sroa_idx.i.i.sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %_95.i.i.i.i, i64 32
  store i64 %_21.sroa.6.i.i.sroa.7.0.copyload.i.i, ptr %_21.sroa.6.i.i.sroa.7.0._93.sroa.4.0._95.sroa_idx.i.i.sroa_idx.i.i, align 8, !noalias !107681
  %_21.sroa.6.i.i.sroa.8.0._93.sroa.4.0._95.sroa_idx.i.i.sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %_95.i.i.i.i, i64 40
  store ptr %_21.sroa.6.i.i.sroa.8.0.copyload.i.i, ptr %_21.sroa.6.i.i.sroa.8.0._93.sroa.4.0._95.sroa_idx.i.i.sroa_idx.i.i, align 8, !noalias !107681
  %_21.sroa.6.i.i.sroa.9.0._93.sroa.4.0._95.sroa_idx.i.i.sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %_95.i.i.i.i, i64 48
  store ptr %_21.sroa.6.i.i.sroa.9.0.copyload.i.i, ptr %_21.sroa.6.i.i.sroa.9.0._93.sroa.4.0._95.sroa_idx.i.i.sroa_idx.i.i, align 8, !noalias !107681
  %_21.sroa.6.i.i.sroa.10.0._93.sroa.4.0._95.sroa_idx.i.i.sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %_95.i.i.i.i, i64 56
  store i32 %_21.sroa.6.i.i.sroa.10.0.copyload.i.i, ptr %_21.sroa.6.i.i.sroa.10.0._93.sroa.4.0._95.sroa_idx.i.i.sroa_idx.i.i, align 8, !noalias !107681
  %_21.sroa.6.i.i.sroa.11.0._93.sroa.4.0._95.sroa_idx.i.i.sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %_95.i.i.i.i, i64 60
  store i32 %_21.sroa.6.i.i.sroa.11.0.copyload.i.i, ptr %_21.sroa.6.i.i.sroa.11.0._93.sroa.4.0._95.sroa_idx.i.i.sroa_idx.i.i, align 4, !noalias !107681
  %_93.sroa.5.0._95.sroa_idx.i.i.i.i = getelementptr inbounds nuw i8, ptr %_95.i.i.i.i, i64 64
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %_93.sroa.5.0._95.sroa_idx.i.i.i.i, ptr noundef nonnull align 8 dereferenceable(32) %_21.sroa.7.i.i.i.i, i64 32, i1 false), !noalias !107681
  call void @llvm.lifetime.start.p0(ptr nonnull %_94.i.i.i.i), !noalias !107681
  store i64 %_21.sroa.0.0.copyload115.i.i.i.i, ptr %_95.i.i.i.i, align 8, !noalias !107681
  call void @llvm.experimental.noalias.scope.decl(metadata !107771)
  call void @llvm.lifetime.start.p0(ptr nonnull %_3.i50.i.i.i.i), !noalias !107681
  call void @llvm.lifetime.start.p0(ptr nonnull %args.i49.i.i.i.i), !noalias !107774
  store ptr %_95.i.i.i.i, ptr %args.i49.i.i.i.i, align 8, !noalias !107774
  %_7.sroa.4.0..sroa_idx.i51.i.i.i.i = getelementptr inbounds nuw i8, ptr %args.i49.i.i.i.i, i64 8
  store ptr @<purrdf_core::diagnostic::RdfDiagnostic as core::fmt::Display>::fmt, ptr %_7.sroa.4.0..sroa_idx.i51.i.i.i.i, align 8, !noalias !107774
; invoke alloc::fmt::format::format_inner
  invoke void @alloc::fmt::format::format_inner(ptr noalias nofree noundef nonnull sret([24 x i8]) align 8 captures(none) dereferenceable(24) %_3.i50.i.i.i.i, ptr noundef nonnull @alloc_592fabd2ffa0e5a6b6713c88c8b00c99, ptr noundef nonnull %args.i49.i.i.i.i)
          to label %bb5.i57.i.i.i.i unwind label %cleanup.i52.i.i.i.i, !noalias !107776

cleanup.i52.i.i.i.i:                              ; preds = %bb4.i.i.i.i.i.i, %bb2.i.i59.i.i.i.i, %bb59.i.i.i.i
  %106 = landingpad { ptr, i32 }
          cleanup
  br label %cleanup.body.i53.i.i.i.i

cleanup.body.i53.i.i.i.i:                         ; preds = %bb2.i.i.i4.i.i.i.i58.i.i.i.i, %bb9.i.i.i.i.i.i, %cleanup.i52.i.i.i.i
  %eh.lpad-body.i54.i.i.i.i = phi { ptr, i32 } [ %106, %cleanup.i52.i.i.i.i ], [ %lpad.thr_comm.split-lp.i.i.i.i.i.i, %bb2.i.i.i4.i.i.i.i58.i.i.i.i ], [ %lpad.thr_comm.split-lp.i.i.i.i.i.i, %bb9.i.i.i.i.i.i ]
; invoke core::ptr::drop_glue::<purrdf_core::diagnostic::RdfDiagnostic>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_core::diagnostic::RdfDiagnostic>(ptr noalias nofree noundef nonnull align 8 dereferenceable(96) %_95.i.i.i.i) #79
          to label %cleanup10.body.i.i.i.i unwind label %terminate.i55.i.i.i.i, !noalias !107776

bb5.i57.i.i.i.i:                                  ; preds = %bb59.i.i.i.i
  call void @llvm.lifetime.end.p0(ptr nonnull %args.i49.i.i.i.i), !noalias !107774
  call void @llvm.experimental.noalias.scope.decl(metadata !107777)
  call void @llvm.experimental.noalias.scope.decl(metadata !107780)
  %107 = getelementptr inbounds nuw i8, ptr %_95.i.i.i.i, i64 80
  %108 = load ptr, ptr %107, align 8, !alias.scope !107782, !noalias !107783, !align !3893, !noundef !3892
  %_20.i.i.i.i.i.i = load ptr, ptr %_93.sroa.4.0._95.sroa_idx.i.i.i.i, align 8, !alias.scope !107782, !noalias !107783, !nonnull !3892, !noundef !3892
  %_19.i.i.i.i.i.i = load i64, ptr %_21.sroa.6.i.i.sroa.6.0._93.sroa.4.0._95.sroa_idx.i.i.sroa_idx.i.i, align 8, !alias.scope !107782, !noalias !107783, !noundef !3892
; invoke purrdf_validate::xpath_regex::diagnostic_refusal_code
  %109 = invoke { ptr, i64 } @purrdf_validate::xpath_regex::diagnostic_refusal_code(ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) %_20.i.i.i.i.i.i, i64 noundef %_19.i.i.i.i.i.i)
          to label %bb1.i.i.i.i.i.i unwind label %bb9.i.i.i.i.i.i, !noalias !107785

bb1.i.i.i.i.i.i:                                  ; preds = %bb5.i57.i.i.i.i
  %110 = extractvalue { ptr, i64 } %109, 0
  %.not3.i.i.i.i.i.i = icmp ne ptr %108, null
  %.not4.i.i.i.i.i.i = icmp eq ptr %110, null
  %or.cond.i.i.i.i.i.i = select i1 %.not3.i.i.i.i.i.i, i1 true, i1 %.not4.i.i.i.i.i.i
  br i1 %or.cond.i.i.i.i.i.i, label %bb2.i.i59.i.i.i.i, label %bb4.i.i.i.i.i.i

bb2.i.i59.i.i.i.i:                                ; preds = %bb1.i.i.i.i.i.i
; invoke purrdf_native::py_store::presentation::presented_value_error
  invoke fastcc void @purrdf_native::py_store::presentation::presented_value_error(ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(48) %_94.i.i.i.i, ptr noalias nofree noundef nonnull readonly align 8 captures(none) dereferenceable(24) %_3.i50.i.i.i.i, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable_or_null(80) %108)
          to label %bb1.i.i.i.i.i unwind label %cleanup.i52.i.i.i.i, !noalias !107681

bb4.i.i.i.i.i.i:                                  ; preds = %bb1.i.i.i.i.i.i
  %111 = extractvalue { ptr, i64 } %109, 1
; invoke purrdf_native::py_store::presentation::refusal_value_error
  invoke fastcc void @purrdf_native::py_store::presentation::refusal_value_error(ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(48) %_94.i.i.i.i, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(24) %_3.i50.i.i.i.i, ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) %110, i64 noundef %111)
          to label %bb1.i.i.i.i.i unwind label %cleanup.i52.i.i.i.i, !noalias !107681

bb9.i.i.i.i.i.i:                                  ; preds = %bb5.i57.i.i.i.i
  %lpad.thr_comm.split-lp.i.i.i.i.i.i = landingpad { ptr, i32 }
          cleanup
  call void @llvm.experimental.noalias.scope.decl(metadata !107786)
  %_1.val.i.i.i.i.i.i.i = load i64, ptr %_3.i50.i.i.i.i, align 8, !alias.scope !107789, !noalias !107790
  %112 = icmp eq i64 %_1.val.i.i.i.i.i.i.i, 0
  br i1 %112, label %cleanup.body.i53.i.i.i.i, label %bb2.i.i.i4.i.i.i.i58.i.i.i.i

bb2.i.i.i4.i.i.i.i58.i.i.i.i:                     ; preds = %bb9.i.i.i.i.i.i
  %113 = getelementptr inbounds nuw i8, ptr %_3.i50.i.i.i.i, i64 8
  %_1.val1.i.i.i.i.i.i.i = load ptr, ptr %113, align 8, !alias.scope !107789, !noalias !107790, !nonnull !3892, !noundef !3892
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %_1.val1.i.i.i.i.i.i.i, i64 noundef %_1.val.i.i.i.i.i.i.i, i64 noundef range(i64 1, -9223372036854775807) 1) #77, !noalias !107791
  br label %cleanup.body.i53.i.i.i.i

bb1.i.i.i.i.i:                                    ; preds = %bb4.i.i.i.i.i.i, %bb2.i.i59.i.i.i.i
; invoke core::ptr::drop_glue::<purrdf_core::diagnostic::RdfDiagnostic>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_core::diagnostic::RdfDiagnostic>(ptr noalias nofree noundef nonnull align 8 dereferenceable(96) %_95.i.i.i.i)
          to label %bb61.i.i.i.i unwind label %cleanup10.i.i.i.i, !noalias !107681

terminate.i55.i.i.i.i:                            ; preds = %cleanup.body.i53.i.i.i.i
  %114 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #75, !noalias !107776
  unreachable

bb60.i.i.i.i:                                     ; preds = %bb57.i.i.i.i
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %_32.sroa.10.i.i, ptr noundef nonnull align 8 dereferenceable(16) %_21.sroa.6.i.i.sroa.6.i.i, i64 16, i1 false), !noalias !107704
  br label %bb58.i.i.i.i

bb58.i.i.i.i:                                     ; preds = %bb61.i.i.i.i, %bb60.i.i.i.i
  %_32.sroa.20.2.i.i = phi i32 [ %_21.sroa.6.i.i.sroa.11.0.copyload.i.i, %bb60.i.i.i.i ], [ %_32.sroa.20.8.copyload177.i.i, %bb61.i.i.i.i ]
  %_32.sroa.19.2.i.i = phi i32 [ %_21.sroa.6.i.i.sroa.10.0.copyload.i.i, %bb60.i.i.i.i ], [ %_32.sroa.19.8.copyload173.i.i, %bb61.i.i.i.i ]
  %_32.sroa.18.2.i.i = phi ptr [ %_21.sroa.6.i.i.sroa.9.0.copyload.i.i, %bb60.i.i.i.i ], [ %_32.sroa.18.8.copyload169.i.i, %bb61.i.i.i.i ]
  %_32.sroa.17.2.i.i = phi ptr [ %_21.sroa.6.i.i.sroa.8.0.copyload.i.i, %bb60.i.i.i.i ], [ %_32.sroa.17.8.copyload165.i.i, %bb61.i.i.i.i ]
  %_32.sroa.16.2.i.i = phi i64 [ %_21.sroa.6.i.i.sroa.7.0.copyload.i.i, %bb60.i.i.i.i ], [ %_32.sroa.16.8.copyload161.i.i, %bb61.i.i.i.i ]
  %_32.sroa.0.2.i.i = phi i64 [ %_21.sroa.6.i.i.sroa.0.0.copyload.i.i, %bb60.i.i.i.i ], [ -1, %bb61.i.i.i.i ]
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::extension_env::ExtensionEnv>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::extension_env::ExtensionEnv>(ptr noalias nofree noundef align 8 dereferenceable(352) %_29.i.i.i.i)
          to label %bb8.i.i.i.i unwind label %cleanup9.i.i.i.i, !noalias !107681

bb61.i.i.i.i:                                     ; preds = %bb1.i.i.i.i.i
  call void @llvm.lifetime.end.p0(ptr nonnull %_3.i50.i.i.i.i), !noalias !107681
  call void @llvm.lifetime.end.p0(ptr nonnull %_95.i.i.i.i), !noalias !107681
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %_32.sroa.10.i.i, ptr noundef nonnull align 8 dereferenceable(16) %_94.i.i.i.i, i64 16, i1 false), !noalias !107704
  %_32.sroa.16.8._94.i.i.sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %_94.i.i.i.i, i64 16
  %_32.sroa.16.8.copyload161.i.i = load i64, ptr %_32.sroa.16.8._94.i.i.sroa_idx.i.i, align 8, !noalias !107704
  %_32.sroa.17.8._94.i.i.sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %_94.i.i.i.i, i64 24
  %_32.sroa.17.8.copyload165.i.i = load ptr, ptr %_32.sroa.17.8._94.i.i.sroa_idx.i.i, align 8, !noalias !107704
  %_32.sroa.18.8._94.i.i.sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %_94.i.i.i.i, i64 32
  %_32.sroa.18.8.copyload169.i.i = load ptr, ptr %_32.sroa.18.8._94.i.i.sroa_idx.i.i, align 8, !noalias !107704
  %_32.sroa.19.8._94.i.i.sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %_94.i.i.i.i, i64 40
  %_32.sroa.19.8.copyload173.i.i = load i32, ptr %_32.sroa.19.8._94.i.i.sroa_idx.i.i, align 8, !noalias !107704
  %_32.sroa.20.8._94.i.i.sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %_94.i.i.i.i, i64 44
  %_32.sroa.20.8.copyload177.i.i = load i32, ptr %_32.sroa.20.8._94.i.i.sroa_idx.i.i, align 4, !noalias !107704
  call void @llvm.lifetime.end.p0(ptr nonnull %_94.i.i.i.i), !noalias !107681
  br label %bb58.i.i.i.i

bb8.i.i.i.i:                                      ; preds = %bb58.i.i.i.i
  call void @llvm.lifetime.end.p0(ptr nonnull %_30.sroa.6.i.i.sroa.0.i.i)
  call void @llvm.lifetime.end.p0(ptr nonnull %_29.i.i.i.i), !noalias !107681
  call void @llvm.lifetime.end.p0(ptr nonnull %_21.sroa.6.i.i.sroa.6.i.i)
  call void @llvm.lifetime.end.p0(ptr nonnull %_21.sroa.7.i.i.i.i)
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::engine::NativeSparqlEngine>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::engine::NativeSparqlEngine>(ptr noalias nofree noundef align 8 dereferenceable(520) %engine.i.i.i.i)
          to label %bb9.i.i.i.i unwind label %bb34.thread135.i.i.i.i, !noalias !107681

bb9.i.i.i.i:                                      ; preds = %bb8.i.i.i.i
  call void @llvm.lifetime.end.p0(ptr nonnull %engine.i.i.i.i), !noalias !107681
  br i1 %.not25.i.i.i.i, label %bb10.i.i.i.i, label %bb2.i63.i.i.i.i

bb2.i63.i.i.i.i:                                  ; preds = %bb9.i.i.i.i
; invoke core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, alloc::sync::Arc<dyn purrdf_sparql_eval::agg_fn::CustomAggregate>)>>
  invoke fastcc void @core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, alloc::sync::Arc<dyn purrdf_sparql_eval::agg_fn::CustomAggregate>)>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(40) %aggregates.i.i.i.i)
          to label %bb10.i.i.i.i unwind label %cleanup6.i.i.i.i, !noalias !107681

bb10.i.i.i.i:                                     ; preds = %bb2.i63.i.i.i.i, %bb9.i.i.i.i
  call void @llvm.lifetime.end.p0(ptr nonnull %aggregates.i.i.i.i), !noalias !107681
  br i1 %.not24.i.i.i.i, label %bb11.i.i.i.i, label %bb2.i67.i.i.i.i

bb2.i67.i.i.i.i:                                  ; preds = %bb10.i.i.i.i
; invoke core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, alloc::sync::Arc<dyn purrdf_sparql_eval::property_fn::PropertyFunction>)>>
  invoke fastcc void @core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, alloc::sync::Arc<dyn purrdf_sparql_eval::property_fn::PropertyFunction>)>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(72) %registry.i.i.i.i)
          to label %core::ptr::drop_glue::<purrdf_sparql_eval::property_fn::PropertyFunctionRegistry> (.exit.i.i.i.i.i) unwind label %cleanup.i.i68.i.i.i.i, !noalias !107681

cleanup.i.i68.i.i.i.i:                            ; preds = %bb2.i67.i.i.i.i
  %115 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, purrdf_sparql_eval::property_fn::RankedDeclaration)>>
  invoke fastcc void @core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, purrdf_sparql_eval::property_fn::RankedDeclaration)>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(32) %_9.sroa.5.i.i.sroa.9.0.registry.i.i.sroa_idx.i.i)
          to label %bb21.i.i.i.i unwind label %terminate.i.i.i.i.i.i, !noalias !107681

terminate.i.i.i.i.i.i:                            ; preds = %cleanup.i.i68.i.i.i.i
  %116 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #75, !noalias !107792
  unreachable

core::ptr::drop_glue::<purrdf_sparql_eval::property_fn::PropertyFunctionRegistry> (.exit.i.i.i.i.i): ; preds = %bb2.i67.i.i.i.i
; invoke core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, purrdf_sparql_eval::property_fn::RankedDeclaration)>>
  invoke fastcc void @core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, purrdf_sparql_eval::property_fn::RankedDeclaration)>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(32) %_9.sroa.5.i.i.sroa.9.0.registry.i.i.sroa_idx.i.i)
          to label %bb11.i.i.i.i unwind label %cleanup5.i.i.i.i, !noalias !107681

bb11.i.i.i.i:                                     ; preds = %core::ptr::drop_glue::<purrdf_sparql_eval::property_fn::PropertyFunctionRegistry> (.exit.i.i.i.i.i), %bb10.i.i.i.i
  call void @llvm.lifetime.end.p0(ptr nonnull %registry.i.i.i.i), !noalias !107681
  call void @llvm.experimental.noalias.scope.decl(metadata !107797)
  call void @llvm.experimental.noalias.scope.decl(metadata !107800)
  %_10.i.i73.i.i.i.i = load ptr, ptr %dataset.i.i.i.i, align 8, !alias.scope !107803, !noalias !107681, !nonnull !3892, !noundef !3892
  %_2.i.i74.i.i.i.i = atomicrmw sub ptr %_10.i.i73.i.i.i.i, i64 1 release, align 8, !noalias !107804
  %117 = icmp eq i64 %_2.i.i74.i.i.i.i, 1
  br i1 %117, label %bb2.i.i75.i.i.i.i, label %bb12.i.i.i.i

bb2.i.i75.i.i.i.i:                                ; preds = %bb11.i.i.i.i
  fence acquire
; invoke <alloc::sync::Arc<purrdf_core::ir::dataset::RdfDataset>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_core::ir::dataset::RdfDataset>>::drop_slow(ptr noalias nofree noundef nonnull align 8 dereferenceable(8) %dataset.i.i.i.i) #81
          to label %bb12.i.i.i.i unwind label %bb40.i.i.i.i, !noalias !107681

bb12.i.i.i.i:                                     ; preds = %bb2.i.i75.i.i.i.i, %bb11.i.i.i.i
  call void @llvm.lifetime.end.p0(ptr nonnull %dataset.i.i.i.i), !noalias !107681
  %118 = icmp sgt i64 %59, 0
  br i1 %118, label %bb2.i.i.i4.i.i.i.i.i.i.i, label %bb16.i.i.i.i

bb2.i.i.i4.i.i.i.i.i.i.i:                         ; preds = %bb12.i.i.i.i
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %_67.i.i.i.i, i64 noundef %59, i64 noundef range(i64 1, -9223372036854775807) 1) #77, !noalias !107805
  br label %bb16.i.i.i.i

bb16.i.i.i.i:                                     ; preds = %bb31.i.i.i.i, %bb30.i.i.i.i, %bb2.i.i.i4.i.i.i.i.i.i.i, %bb12.i.i.i.i
  %_32.sroa.20.1.i.i = phi i32 [ %_32.sroa.20.0.i.i, %bb31.i.i.i.i ], [ %_32.sroa.20.0.i.i, %bb30.i.i.i.i ], [ %_32.sroa.20.2.i.i, %bb2.i.i.i4.i.i.i.i.i.i.i ], [ %_32.sroa.20.2.i.i, %bb12.i.i.i.i ]
  %_32.sroa.19.1.i.i = phi i32 [ %_32.sroa.19.0.i.i, %bb31.i.i.i.i ], [ %_32.sroa.19.0.i.i, %bb30.i.i.i.i ], [ %_32.sroa.19.2.i.i, %bb2.i.i.i4.i.i.i.i.i.i.i ], [ %_32.sroa.19.2.i.i, %bb12.i.i.i.i ]
  %_32.sroa.18.1.i.i = phi ptr [ %_32.sroa.18.0.i.i, %bb31.i.i.i.i ], [ %_32.sroa.18.0.i.i, %bb30.i.i.i.i ], [ %_32.sroa.18.2.i.i, %bb2.i.i.i4.i.i.i.i.i.i.i ], [ %_32.sroa.18.2.i.i, %bb12.i.i.i.i ]
  %_32.sroa.17.1.i.i = phi ptr [ %_32.sroa.17.0.i.i, %bb31.i.i.i.i ], [ %_32.sroa.17.0.i.i, %bb30.i.i.i.i ], [ %_32.sroa.17.2.i.i, %bb2.i.i.i4.i.i.i.i.i.i.i ], [ %_32.sroa.17.2.i.i, %bb12.i.i.i.i ]
  %_32.sroa.16.1.i.i = phi i64 [ %_32.sroa.16.0.i.i, %bb31.i.i.i.i ], [ %_32.sroa.16.0.i.i, %bb30.i.i.i.i ], [ %_32.sroa.16.2.i.i, %bb2.i.i.i4.i.i.i.i.i.i.i ], [ %_32.sroa.16.2.i.i, %bb12.i.i.i.i ]
  %_32.sroa.0.1.i.i = phi i64 [ -1, %bb31.i.i.i.i ], [ -1, %bb30.i.i.i.i ], [ %_32.sroa.0.2.i.i, %bb2.i.i.i4.i.i.i.i.i.i.i ], [ %_32.sroa.0.2.i.i, %bb12.i.i.i.i ]
; invoke core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_core::ir::term::TermValue)>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_core::ir::term::TermValue)>>(ptr noalias nofree noundef readonly align 8 dereferenceable(24) %41)
          to label %bb2.i48.i.i unwind label %cleanup1.i.i.i, !noalias !107810

terminate.i.i.i.i:                                ; preds = %bb35.i.i.i.i, %bb39.i.i.i.i, %bb27.i.i.i.i, %cleanup10.body.i.i.i.i, %bb18.i.i.i.i, %bb2.i.i.i.i.i, %bb20.i.i.i.i, %bb2.i.i.i.i.i.i
  %119 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #75, !noalias !107681
  unreachable

bb13.i.i.i.i:                                     ; preds = %bb55.i.i.i.i
  call void @llvm.lifetime.end.p0(ptr nonnull %engine.i.i.i.i), !noalias !107681
  br i1 %.not25.i.i.i.i, label %bb14.i.i.i.i, label %bb2.i80.i.i.i.i

bb2.i80.i.i.i.i:                                  ; preds = %bb13.i.i.i.i
; invoke core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, alloc::sync::Arc<dyn purrdf_sparql_eval::agg_fn::CustomAggregate>)>>
  invoke fastcc void @core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, alloc::sync::Arc<dyn purrdf_sparql_eval::agg_fn::CustomAggregate>)>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(40) %aggregates.i.i.i.i)
          to label %bb14.i.i.i.i unwind label %cleanup6.i.i.i.i, !noalias !107681

bb14.i.i.i.i:                                     ; preds = %bb2.i80.i.i.i.i, %bb13.i.i.i.i
  call void @llvm.lifetime.end.p0(ptr nonnull %aggregates.i.i.i.i), !noalias !107681
  br i1 %.not24.i.i.i.i, label %bb15.i.i.i.i, label %bb2.i84.i.i.i.i

bb2.i84.i.i.i.i:                                  ; preds = %bb14.i.i.i.i
; invoke core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, alloc::sync::Arc<dyn purrdf_sparql_eval::property_fn::PropertyFunction>)>>
  invoke fastcc void @core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, alloc::sync::Arc<dyn purrdf_sparql_eval::property_fn::PropertyFunction>)>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(72) %registry.i.i.i.i)
          to label %core::ptr::drop_glue::<purrdf_sparql_eval::property_fn::PropertyFunctionRegistry> (.exit.i88.i.i.i.i) unwind label %cleanup.i.i85.i.i.i.i, !noalias !107681

cleanup.i.i85.i.i.i.i:                            ; preds = %bb2.i84.i.i.i.i
  %120 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, purrdf_sparql_eval::property_fn::RankedDeclaration)>>
  invoke fastcc void @core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, purrdf_sparql_eval::property_fn::RankedDeclaration)>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(32) %_9.sroa.5.i.i.sroa.9.0.registry.i.i.sroa_idx.i.i)
          to label %bb21.i.i.i.i unwind label %terminate.i.i86.i.i.i.i, !noalias !107681

terminate.i.i86.i.i.i.i:                          ; preds = %cleanup.i.i85.i.i.i.i
  %121 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #75, !noalias !107811
  unreachable

core::ptr::drop_glue::<purrdf_sparql_eval::property_fn::PropertyFunctionRegistry> (.exit.i88.i.i.i.i): ; preds = %bb2.i84.i.i.i.i
; invoke core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, purrdf_sparql_eval::property_fn::RankedDeclaration)>>
  invoke fastcc void @core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, purrdf_sparql_eval::property_fn::RankedDeclaration)>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(32) %_9.sroa.5.i.i.sroa.9.0.registry.i.i.sroa_idx.i.i)
          to label %bb15.i.i.i.i unwind label %cleanup5.i.i.i.i, !noalias !107681

bb15.i.i.i.i:                                     ; preds = %core::ptr::drop_glue::<purrdf_sparql_eval::property_fn::PropertyFunctionRegistry> (.exit.i88.i.i.i.i), %bb14.i.i.i.i, %bb44.i.i.i.i
  %_32.sroa.20.3.i.i = phi i32 [ %_9.sroa.5.i.i.sroa.11.0.copyload204.i.i, %bb44.i.i.i.i ], [ %_30.sroa.6.i.i.sroa.10.0.copyload.i.i, %bb14.i.i.i.i ], [ %_30.sroa.6.i.i.sroa.10.0.copyload.i.i, %core::ptr::drop_glue::<purrdf_sparql_eval::property_fn::PropertyFunctionRegistry> (.exit.i88.i.i.i.i) ]
  %_32.sroa.19.3.i.i = phi i32 [ %_9.sroa.5.i.i.sroa.10.0.copyload201.i.i, %bb44.i.i.i.i ], [ %_30.sroa.6.i.i.sroa.9.0.copyload.i.i, %bb14.i.i.i.i ], [ %_30.sroa.6.i.i.sroa.9.0.copyload.i.i, %core::ptr::drop_glue::<purrdf_sparql_eval::property_fn::PropertyFunctionRegistry> (.exit.i88.i.i.i.i) ]
  %_32.sroa.18.3.i.i = phi ptr [ %_9.sroa.5.i.i.sroa.9.0.copyload198.i.i, %bb44.i.i.i.i ], [ %_30.sroa.6.i.i.sroa.8.0.copyload.i.i, %bb14.i.i.i.i ], [ %_30.sroa.6.i.i.sroa.8.0.copyload.i.i, %core::ptr::drop_glue::<purrdf_sparql_eval::property_fn::PropertyFunctionRegistry> (.exit.i88.i.i.i.i) ]
  %_32.sroa.17.3.i.i = phi ptr [ %_9.sroa.5.i.i.sroa.8.0.copyload195.i.i, %bb44.i.i.i.i ], [ %_30.sroa.6.i.i.sroa.7.0.copyload.i.i, %bb14.i.i.i.i ], [ %_30.sroa.6.i.i.sroa.7.0.copyload.i.i, %core::ptr::drop_glue::<purrdf_sparql_eval::property_fn::PropertyFunctionRegistry> (.exit.i88.i.i.i.i) ]
  %_32.sroa.16.3.i.i = phi i64 [ %_9.sroa.5.i.i.sroa.7.0.copyload192.i.i, %bb44.i.i.i.i ], [ %_30.sroa.6.i.i.sroa.6.0.copyload.i.i, %bb14.i.i.i.i ], [ %_30.sroa.6.i.i.sroa.6.0.copyload.i.i, %core::ptr::drop_glue::<purrdf_sparql_eval::property_fn::PropertyFunctionRegistry> (.exit.i88.i.i.i.i) ]
  %_40.sroa.0.7.i.i.i.i = phi i8 [ 1, %bb44.i.i.i.i ], [ 0, %bb14.i.i.i.i ], [ 0, %core::ptr::drop_glue::<purrdf_sparql_eval::property_fn::PropertyFunctionRegistry> (.exit.i88.i.i.i.i) ]
  call void @llvm.lifetime.end.p0(ptr nonnull %registry.i.i.i.i), !noalias !107681
  call void @llvm.experimental.noalias.scope.decl(metadata !107816)
  call void @llvm.experimental.noalias.scope.decl(metadata !107819)
  %_10.i.i95.i.i.i.i = load ptr, ptr %dataset.i.i.i.i, align 8, !alias.scope !107822, !noalias !107681, !nonnull !3892, !noundef !3892
  %_2.i.i96.i.i.i.i = atomicrmw sub ptr %_10.i.i95.i.i.i.i, i64 1 release, align 8, !noalias !107823
  %122 = icmp eq i64 %_2.i.i96.i.i.i.i, 1
  br i1 %122, label %bb2.i.i97.i.i.i.i, label %bb62.i.i.i.i

bb2.i.i97.i.i.i.i:                                ; preds = %bb15.i.i.i.i
  fence acquire
; invoke <alloc::sync::Arc<purrdf_core::ir::dataset::RdfDataset>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_core::ir::dataset::RdfDataset>>::drop_slow(ptr noalias nofree noundef nonnull align 8 dereferenceable(8) %dataset.i.i.i.i) #81
          to label %bb62.i.i.i.i unwind label %bb40.i.i.i.i, !noalias !107681

bb33.i.i.i.i:                                     ; preds = %bb5.i.i.i.i
  %lpad.thr_comm.split-lp134.i.i.i.i = landingpad { ptr, i32 }
          cleanup
; call core::ptr::drop_glue::<purrdf_sparql_algebra::parser::ParserOptions>
  call fastcc void @core::ptr::drop_glue::<purrdf_sparql_algebra::parser::ParserOptions>(ptr noalias nofree noundef align 8 dereferenceable(72) %parser_options.i.i.i.i) #79, !noalias !107681
  br label %bb19.i.i.i.i

bb62.i.i.i.i:                                     ; preds = %bb2.i.i97.i.i.i.i, %bb15.i.i.i.i
  call void @llvm.lifetime.end.p0(ptr nonnull %dataset.i.i.i.i), !noalias !107681
  %123 = trunc nuw i8 %_40.sroa.0.7.i.i.i.i to i1
  br label %bb32.i.i.i.i

bb32.i.i.i.i:                                     ; preds = %bb2.i.i.i6.i.i.i.i, %bb4.i7.i.i.i, %bb62.i.i.i.i
  %_32.sroa.20.0.i.i = phi i32 [ %_32.sroa.20.3.i.i, %bb62.i.i.i.i ], [ undef, %bb4.i7.i.i.i ], [ undef, %bb2.i.i.i6.i.i.i.i ]
  %_32.sroa.19.0.i.i = phi i32 [ %_32.sroa.19.3.i.i, %bb62.i.i.i.i ], [ 3, %bb4.i7.i.i.i ], [ 3, %bb2.i.i.i6.i.i.i.i ]
  %_32.sroa.18.0.i.i = phi ptr [ %_32.sroa.18.3.i.i, %bb62.i.i.i.i ], [ @vtable.1R, %bb4.i7.i.i.i ], [ @vtable.1R, %bb2.i.i.i6.i.i.i.i ]
  %_32.sroa.17.0.i.i = phi ptr [ %_32.sroa.17.3.i.i, %bb62.i.i.i.i ], [ %49, %bb4.i7.i.i.i ], [ %49, %bb2.i.i.i6.i.i.i.i ]
  %_32.sroa.16.0.i.i = phi i64 [ %_32.sroa.16.3.i.i, %bb62.i.i.i.i ], [ 1, %bb4.i7.i.i.i ], [ 1, %bb2.i.i.i6.i.i.i.i ]
  %_40.sroa.0.8.i.i.i.i = phi i1 [ %123, %bb62.i.i.i.i ], [ true, %bb4.i7.i.i.i ], [ true, %bb2.i.i.i6.i.i.i.i ]
  call void @llvm.experimental.noalias.scope.decl(metadata !107824)
  %124 = load i64, ptr %37, align 8, !range !3909, !alias.scope !107827, !noalias !107695, !noundef !3892
  %125 = icmp eq i64 %124, -1
  br i1 %125, label %bb30.i.i.i.i, label %bb2.i100.i.i.i.i

bb2.i100.i.i.i.i:                                 ; preds = %bb32.i.i.i.i
  call void @llvm.experimental.noalias.scope.decl(metadata !107828)
  %126 = icmp eq i64 %124, 0
  br i1 %126, label %bb30.i.i.i.i, label %bb2.i.i.i4.i.i.i101.i.i.i.i

bb2.i.i.i4.i.i.i101.i.i.i.i:                      ; preds = %bb2.i100.i.i.i.i
  %127 = getelementptr inbounds nuw i8, ptr %_33.i.i, i64 56
  %_1.val1.i.i102.i.i.i.i = load ptr, ptr %127, align 8, !alias.scope !107831, !noalias !107695, !nonnull !3892, !noundef !3892
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %_1.val1.i.i102.i.i.i.i, i64 noundef %124, i64 noundef range(i64 1, -9223372036854775807) 1) #77, !noalias !107832
  br label %bb30.i.i.i.i

bb43.i.i.i.i:                                     ; preds = %bb8.i.i.i.i.i
  call void @llvm.lifetime.end.p0(ptr nonnull %_3.i.i.i.i.i), !noalias !107681
  call void @llvm.lifetime.end.p0(ptr nonnull %_47.i.i.i.i), !noalias !107681
  call void @llvm.lifetime.end.p0(ptr nonnull %_5.i.i.i.i), !noalias !107681
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %_32.sroa.10.i.i, i8 0, i64 16, i1 false), !alias.scope !107833, !noalias !107704
  call void @llvm.lifetime.end.p0(ptr nonnull %dataset.i.i.i.i), !noalias !107681
  call void @llvm.experimental.noalias.scope.decl(metadata !107834)
  %128 = getelementptr inbounds nuw i8, ptr %_33.i.i, i64 8
  %_1.val.i.i.i.i = load ptr, ptr %128, align 8, !alias.scope !107837, !noalias !107695, !nonnull !3892, !noundef !3892
  %129 = getelementptr inbounds nuw i8, ptr %_33.i.i, i64 16
  %_1.val1.i.i47.i.i = load i64, ptr %129, align 8, !alias.scope !107837, !noalias !107695, !noundef !3892
  %_7.i.i.i.i.i.i24 = icmp eq i64 %_1.val1.i.i47.i.i, 0
  br i1 %_7.i.i.i.i.i.i24, label %bb4.i7.i.i.i, label %bb5.i.i.i.i.i.i

bb6.i.i.i.i.i.i:                                  ; preds = %bb5.i.i.i.i.i.i
  %_7.i.i.i.i.i.i = icmp eq i64 %130, %_1.val1.i.i47.i.i
  br i1 %_7.i.i.i.i.i.i, label %bb4.i7.i.i.i, label %bb5.i.i.i.i.i.i

bb5.i.i.i.i.i.i:                                  ; preds = %bb43.i.i.i.i, %bb6.i.i.i.i.i.i
  %_3.sroa.0.0.i.i.i.i.i.i25 = phi i64 [ %130, %bb6.i.i.i.i.i.i ], [ 0, %bb43.i.i.i.i ]
  %_6.i.i.i.i.i.i = getelementptr inbounds nuw [160 x i8], ptr %_1.val.i.i.i.i, i64 %_3.sroa.0.0.i.i.i.i.i.i25
  %130 = add nuw nsw i64 %_3.sroa.0.0.i.i.i.i.i.i25, 1
; invoke core::ptr::drop_glue::<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>
  invoke fastcc void @core::ptr::drop_glue::<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>(ptr noalias nofree noundef align 8 dereferenceable(160) %_6.i.i.i.i.i.i)
          to label %bb6.i.i.i.i.i.i unwind label %cleanup.i.i.i4.i.i.i, !noalias !107838

bb4.i.i.i5.i.i.i:                                 ; preds = %bb3.i.i.i.i.i.i
  %131 = add i64 %_3.sroa.0.1.i.i.i.i.i.i27, 1
  %_5.i.i.i.i.i.i = icmp eq i64 %131, %_1.val1.i.i47.i.i
  br i1 %_5.i.i.i.i.i.i, label %cleanup.body.i.i.i.i, label %bb3.i.i.i.i.i.i

cleanup.i.i.i4.i.i.i:                             ; preds = %bb5.i.i.i.i.i.i
  %132 = landingpad { ptr, i32 }
          cleanup
  %_5.i.i.i.i.i.i26 = icmp eq i64 %130, %_1.val1.i.i47.i.i
  br i1 %_5.i.i.i.i.i.i26, label %cleanup.body.i.i.i.i, label %bb3.i.i.i.i.i.i

bb3.i.i.i.i.i.i:                                  ; preds = %cleanup.i.i.i4.i.i.i, %bb4.i.i.i5.i.i.i
  %_3.sroa.0.1.i.i.i.i.i.i27 = phi i64 [ %131, %bb4.i.i.i5.i.i.i ], [ %130, %cleanup.i.i.i4.i.i.i ]
  %_4.i.i.i.i.i.i = getelementptr inbounds nuw [160 x i8], ptr %_1.val.i.i.i.i, i64 %_3.sroa.0.1.i.i.i.i.i.i27
; invoke core::ptr::drop_glue::<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>
  invoke fastcc void @core::ptr::drop_glue::<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>(ptr noalias nofree noundef align 8 dereferenceable(160) %_4.i.i.i.i.i.i) #79
          to label %bb4.i.i.i5.i.i.i unwind label %terminate.i.i.i6.i.i.i, !noalias !107838

terminate.i.i.i6.i.i.i:                           ; preds = %bb3.i.i.i.i.i.i
  %133 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #75, !noalias !107838
  unreachable

cleanup.body.i.i.i.i:                             ; preds = %bb4.i.i.i5.i.i.i, %cleanup.i.i.i4.i.i.i
  %_1.val2.i.i.i.i = load i64, ptr %_33.i.i, align 8, !alias.scope !107837, !noalias !107695
  %134 = icmp eq i64 %_1.val2.i.i.i.i, 0
  br i1 %134, label %cleanup12.i.body.i.i.i, label %bb2.i.i.i.i.i.i.i

bb2.i.i.i.i.i.i.i:                                ; preds = %cleanup.body.i.i.i.i
  %alloc_size.i.i.i.i.i.i.i.i = mul nuw i64 %_1.val2.i.i.i.i, 160
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %_1.val.i.i.i.i, i64 noundef %alloc_size.i.i.i.i.i.i.i.i, i64 noundef range(i64 1, -9223372036854775807) 8) #77, !noalias !107838
  br label %cleanup12.i.body.i.i.i

bb4.i7.i.i.i:                                     ; preds = %bb6.i.i.i.i.i.i, %bb43.i.i.i.i
  %_1.val4.i.i.i.i = load i64, ptr %_33.i.i, align 8, !alias.scope !107837, !noalias !107695
  %135 = icmp eq i64 %_1.val4.i.i.i.i, 0
  br i1 %135, label %bb32.i.i.i.i, label %bb2.i.i.i6.i.i.i.i

bb2.i.i.i6.i.i.i.i:                               ; preds = %bb4.i7.i.i.i
  %alloc_size.i.i.i.i7.i.i.i.i = mul nuw i64 %_1.val4.i.i.i.i, 160
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %_1.val.i.i.i.i, i64 noundef %alloc_size.i.i.i.i7.i.i.i.i, i64 noundef range(i64 1, -9223372036854775807) 8) #77, !noalias !107838
  br label %bb32.i.i.i.i

cleanup12.i.body.i.i.i:                           ; preds = %bb2.i.i.i.i.i.i.i, %cleanup.body.i.i.i.i
  call void @llvm.experimental.noalias.scope.decl(metadata !107839)
  %136 = load i64, ptr %37, align 8, !range !3909, !alias.scope !107842, !noalias !107695, !noundef !3892
  %137 = icmp eq i64 %136, -1
  br i1 %137, label %bb27.i.i.i.i, label %bb2.i105.i.i.i.i

bb2.i105.i.i.i.i:                                 ; preds = %cleanup12.i.body.i.i.i
  call void @llvm.experimental.noalias.scope.decl(metadata !107843)
  %138 = icmp eq i64 %136, 0
  br i1 %138, label %bb27.i.i.i.i, label %bb2.i.i.i4.i.i.i106.i.i.i.i

bb2.i.i.i4.i.i.i106.i.i.i.i:                      ; preds = %bb2.i105.i.i.i.i
  %139 = getelementptr inbounds nuw i8, ptr %_33.i.i, i64 56
  %_1.val1.i.i107.i.i.i.i = load ptr, ptr %139, align 8, !alias.scope !107846, !noalias !107695, !nonnull !3892, !noundef !3892
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %_1.val1.i.i107.i.i.i.i, i64 noundef %136, i64 noundef range(i64 1, -9223372036854775807) 1) #77, !noalias !107847
  br label %bb27.i.i.i.i

bb30.i.i.i.i:                                     ; preds = %bb2.i.i.i4.i.i.i101.i.i.i.i, %bb2.i100.i.i.i.i, %bb32.i.i.i.i
  br i1 %_40.sroa.0.8.i.i.i.i, label %bb31.i.i.i.i, label %bb16.i.i.i.i

bb31.i.i.i.i:                                     ; preds = %bb30.i.i.i.i
; call core::ptr::drop_glue::<purrdf_native::py_store::query::EngineConfig>
  call fastcc void @core::ptr::drop_glue::<purrdf_native::py_store::query::EngineConfig>(ptr noalias nofree noundef readonly align 8 dereferenceable(168) %38), !noalias !107695
  br label %bb16.i.i.i.i

bb27.i.i.i.i:                                     ; preds = %bb2.i.i.i4.i.i.i106.i.i.i.i, %bb2.i105.i.i.i.i, %cleanup12.i.body.i.i.i
; call core::ptr::drop_glue::<purrdf_native::py_store::query::EngineConfig>
  call fastcc void @core::ptr::drop_glue::<purrdf_native::py_store::query::EngineConfig>(ptr noalias nofree noundef readonly align 8 dereferenceable(168) %38) #79, !noalias !107695
; invoke core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_core::ir::term::TermValue)>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_core::ir::term::TermValue)>>(ptr noalias nofree noundef readonly align 8 dereferenceable(24) %41) #79
          to label %cleanup1.body.i.i.i unwind label %terminate.i.i.i.i, !noalias !107695

bb38.i.i.i.i:                                     ; preds = %bb39.i.i.i.i, %bb2.i.i.i.i.i.i, %bb21.i.i.i.i, %bb40.i.i.i.i
  %.pn35120.i.i.i.i = phi { ptr, i32 } [ %lpad.thr_comm.split-lp.i.i.i.i, %bb40.i.i.i.i ], [ %eh.lpad-body126.i.i.i.i, %bb39.i.i.i.i ], [ %.pn33.i.i.i.i, %bb21.i.i.i.i ], [ %.pn33.i.i.i.i, %bb2.i.i.i.i.i.i ]
  %_40.sroa.0.0119.i.i.i.i = phi i8 [ %_40.sroa.0.1.ph.i.i.i.i, %bb40.i.i.i.i ], [ 1, %bb39.i.i.i.i ], [ %_40.sroa.0.2.i.i.i.i, %bb21.i.i.i.i ], [ %_40.sroa.0.2.i.i.i.i, %bb2.i.i.i.i.i.i ]
  call void @llvm.experimental.noalias.scope.decl(metadata !107848)
  %140 = load i64, ptr %37, align 8, !range !3909, !alias.scope !107851, !noalias !107695, !noundef !3892
  %141 = icmp eq i64 %140, -1
  br i1 %141, label %bb36.i.i.i.i, label %bb2.i110.i.i.i.i

bb2.i110.i.i.i.i:                                 ; preds = %bb38.i.i.i.i
  call void @llvm.experimental.noalias.scope.decl(metadata !107852)
  %142 = icmp eq i64 %140, 0
  br i1 %142, label %bb36.i.i.i.i, label %bb2.i.i.i4.i.i.i111.i.i.i.i

bb2.i.i.i4.i.i.i111.i.i.i.i:                      ; preds = %bb2.i110.i.i.i.i
  %143 = getelementptr inbounds nuw i8, ptr %_33.i.i, i64 56
  %_1.val1.i.i112.i.i.i.i = load ptr, ptr %143, align 8, !alias.scope !107855, !noalias !107695, !nonnull !3892, !noundef !3892
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %_1.val1.i.i112.i.i.i.i, i64 noundef %140, i64 noundef range(i64 1, -9223372036854775807) 1) #77, !noalias !107856
  br label %bb36.i.i.i.i

bb39.i.i.i.i:                                     ; preds = %cleanup.body.i.i.i.i.i, %bb40.thread127.i.i.i.i
  %eh.lpad-body126.i.i.i.i = phi { ptr, i32 } [ %lpad.thr_comm.i.i.i.i, %bb40.thread127.i.i.i.i ], [ %eh.lpad-body.i.i.i.i.i, %cleanup.body.i.i.i.i.i ]
; invoke core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(272) %_33.i.i) #79
          to label %bb38.i.i.i.i unwind label %terminate.i.i.i.i, !noalias !107695

bb36.i.i.i.i:                                     ; preds = %bb2.i.i.i4.i.i.i111.i.i.i.i, %bb2.i110.i.i.i.i, %bb38.i.i.i.i
  %144 = trunc nuw i8 %_40.sroa.0.0119.i.i.i.i to i1
  br i1 %144, label %bb37.i.i.i.i, label %bb35.i.i.i.i

bb35.i.i.i.i:                                     ; preds = %bb37.i.i.i.i, %bb36.i.i.i.i
; invoke core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_core::ir::term::TermValue)>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_core::ir::term::TermValue)>>(ptr noalias nofree noundef readonly align 8 dereferenceable(24) %41) #79
          to label %cleanup1.body.i.i.i unwind label %terminate.i.i.i.i, !noalias !107695

bb37.i.i.i.i:                                     ; preds = %bb36.i.i.i.i
; call core::ptr::drop_glue::<purrdf_native::py_store::query::EngineConfig>
  call fastcc void @core::ptr::drop_glue::<purrdf_native::py_store::query::EngineConfig>(ptr noalias nofree noundef readonly align 8 dereferenceable(168) %38) #79, !noalias !107695
  br label %bb35.i.i.i.i

cleanup1.i.i.i:                                   ; preds = %bb16.i.i.i.i
  %145 = landingpad { ptr, i32 }
          cleanup
  br label %cleanup1.body.i.i.i

cleanup1.body.i.i.i:                              ; preds = %cleanup1.i.i.i, %bb35.i.i.i.i, %bb27.i.i.i.i
  %eh.lpad-body.i.i.i = phi { ptr, i32 } [ %145, %cleanup1.i.i.i ], [ %132, %bb27.i.i.i.i ], [ %.pn35120.i.i.i.i, %bb35.i.i.i.i ]
; invoke <pyo3::internal::state::SuspendAttach as core::ops::drop::Drop>::drop
  invoke void @<pyo3::internal::state::SuspendAttach as core::ops::drop::Drop>::drop(ptr noalias nofree noundef nonnull align 8 dereferenceable(16) %_guard.i.i.i)
          to label %bb47.i.i unwind label %terminate.i.i.i, !noalias !107675

bb2.i48.i.i:                                      ; preds = %bb16.i.i.i.i
  call void @llvm.lifetime.end.p0(ptr nonnull %parser_options.i.i.i.i), !noalias !107675
; invoke <pyo3::internal::state::SuspendAttach as core::ops::drop::Drop>::drop
  invoke void @<pyo3::internal::state::SuspendAttach as core::ops::drop::Drop>::drop(ptr noalias nofree noundef nonnull align 8 dereferenceable(16) %_guard.i.i.i)
          to label %bb6.i.i unwind label %cleanup9.i.i, !noalias !107652

terminate.i.i.i:                                  ; preds = %bb6.i.i.i, %cleanup1.body.i.i.i
  %146 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #75, !noalias !107675
  unreachable

bb6.i.i.i:                                        ; preds = %bb58.i.i
  %147 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<<purrdf_native::py_store::quad_store::PyQuadStore>::query_governed::{closure#0}::{closure#0}>
  invoke fastcc void @core::ptr::drop_glue::<<purrdf_native::py_store::quad_store::PyQuadStore>::query_governed::{closure#0}::{closure#0}>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(272) %_33.i.i) #79
          to label %bb47.i.i unwind label %terminate.i.i.i, !noalias !107810

cleanup9.i.i:                                     ; preds = %bb60.i.i, %bb2.i48.i.i
  %148 = landingpad { ptr, i32 }
          cleanup
  br label %bb47.i.i

bb6.i.i:                                          ; preds = %bb2.i48.i.i
  call void @llvm.lifetime.end.p0(ptr nonnull %_guard.i.i.i), !noalias !107675
  call void @llvm.lifetime.end.p0(ptr nonnull %_33.i.i), !noalias !107652
  %149 = icmp eq i64 %_32.sroa.0.1.i.i, -1
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %_31.sroa.6.sroa.0.i.i, ptr noundef nonnull align 8 dereferenceable(16) %_32.sroa.10.i.i, i64 16, i1 false), !noalias !107652
  call void @llvm.lifetime.end.p0(ptr nonnull %_32.sroa.10.i.i)
  br i1 %149, label %bb59.i.i, label %bb60.i.i

bb59.i.i:                                         ; preds = %bb6.i.i
  %150 = getelementptr inbounds nuw i8, ptr %_0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %150, ptr noundef nonnull align 8 dereferenceable(16) %_31.sroa.6.sroa.0.i.i, i64 16, i1 false), !noalias !107653
  %_84.sroa.4.0..sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %_0, i64 24
  store i64 %_32.sroa.16.1.i.i, ptr %_84.sroa.4.0..sroa_idx.i.i, align 8, !alias.scope !107654, !noalias !107653
  %_84.sroa.5.0..sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %_0, i64 32
  store ptr %_32.sroa.17.1.i.i, ptr %_84.sroa.5.0..sroa_idx.i.i, align 8, !alias.scope !107654, !noalias !107653
  %_84.sroa.6.0..sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %_0, i64 40
  store ptr %_32.sroa.18.1.i.i, ptr %_84.sroa.6.0..sroa_idx.i.i, align 8, !alias.scope !107654, !noalias !107653
  %_84.sroa.7.0..sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %_0, i64 48
  store i32 %_32.sroa.19.1.i.i, ptr %_84.sroa.7.0..sroa_idx.i.i, align 8, !alias.scope !107654, !noalias !107653
  %_84.sroa.8.0..sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %_0, i64 52
  store i32 %_32.sroa.20.1.i.i, ptr %_84.sroa.8.0..sroa_idx.i.i, align 4, !alias.scope !107654, !noalias !107653
  store i64 1, ptr %_0, align 8, !alias.scope !107654, !noalias !107653
  call void @llvm.lifetime.end.p0(ptr nonnull %_31.sroa.6.sroa.0.i.i)
  call void @llvm.lifetime.end.p0(ptr nonnull %config.sroa.0.i.i)
  call void @llvm.lifetime.end.p0(ptr nonnull %specs.i.i), !noalias !107652
  br label %bb11.i.i

bb60.i.i:                                         ; preds = %bb6.i.i
  store i64 %_32.sroa.0.1.i.i, ptr %result.i.i, align 8, !noalias !107652
  %_31.sroa.6.0.result.sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %result.i.i, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %_31.sroa.6.0.result.sroa_idx.i.i, ptr noundef nonnull align 8 dereferenceable(16) %_31.sroa.6.sroa.0.i.i, i64 16, i1 false), !noalias !107652
  %_31.sroa.6.sroa.7.0._31.sroa.6.0.result.sroa_idx.sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %result.i.i, i64 24
  store i64 %_32.sroa.16.1.i.i, ptr %_31.sroa.6.sroa.7.0._31.sroa.6.0.result.sroa_idx.sroa_idx.i.i, align 8, !noalias !107652
  %_31.sroa.6.sroa.8.0._31.sroa.6.0.result.sroa_idx.sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %result.i.i, i64 32
  store ptr %_32.sroa.17.1.i.i, ptr %_31.sroa.6.sroa.8.0._31.sroa.6.0.result.sroa_idx.sroa_idx.i.i, align 8, !noalias !107652
  %_31.sroa.6.sroa.9.0._31.sroa.6.0.result.sroa_idx.sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %result.i.i, i64 40
  store ptr %_32.sroa.18.1.i.i, ptr %_31.sroa.6.sroa.9.0._31.sroa.6.0.result.sroa_idx.sroa_idx.i.i, align 8, !noalias !107652
  %_31.sroa.6.sroa.10.0._31.sroa.6.0.result.sroa_idx.sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %result.i.i, i64 48
  store i32 %_32.sroa.19.1.i.i, ptr %_31.sroa.6.sroa.10.0._31.sroa.6.0.result.sroa_idx.sroa_idx.i.i, align 8, !noalias !107652
  %_31.sroa.6.sroa.11.0._31.sroa.6.0.result.sroa_idx.sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %result.i.i, i64 52
  store i32 %_32.sroa.20.1.i.i, ptr %_31.sroa.6.sroa.11.0._31.sroa.6.0.result.sroa_idx.sroa_idx.i.i, align 4, !noalias !107652
  call void @llvm.lifetime.end.p0(ptr nonnull %_31.sroa.6.sroa.0.i.i)
; invoke purrdf_native::py_store::query::materialize_results
  invoke fastcc void @purrdf_native::py_store::query::materialize_results(ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(56) %_0, ptr noalias nofree noundef align 8 captures(address) dereferenceable(56) %result.i.i)
          to label %bb7.i.i unwind label %cleanup9.i.i, !noalias !107653

bb7.i.i:                                          ; preds = %bb60.i.i
  call void @llvm.lifetime.end.p0(ptr nonnull %config.sroa.0.i.i)
  call void @llvm.lifetime.end.p0(ptr nonnull %specs.i.i), !noalias !107652
  call void @llvm.lifetime.end.p0(ptr nonnull %subs.i.i), !noalias !107652
  %151 = icmp sgt i64 %13, 0
  br i1 %151, label %bb2.i.i.i4.i.i.i51.i.i, label %<purrdf_native::py_store::quad_store::PyQuadStore>::query::{closure#0} (.exit.i)

bb2.i.i.i4.i.i.i51.i.i:                           ; preds = %bb7.i.i
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %_50.i.i, i64 noundef %13, i64 noundef range(i64 1, -9223372036854775807) 1) #77, !noalias !107857
  br label %<purrdf_native::py_store::quad_store::PyQuadStore>::query::{closure#0} (.exit.i)

bb11.i.i:                                         ; preds = %bb62.i.i, %bb59.i.i
  %_39.sroa.0.4.i.i = phi i1 [ true, %bb62.i.i ], [ false, %bb59.i.i ]
  %_42.sroa.0.4.i.i = phi i8 [ %_42.sroa.0.5.i.i, %bb62.i.i ], [ 0, %bb59.i.i ]
  call void @llvm.lifetime.end.p0(ptr nonnull %subs.i.i), !noalias !107652
  %152 = icmp sgt i64 %13, 0
  br i1 %152, label %bb2.i.i.i4.i.i.i56.i.i, label %bb33.i.i

bb2.i.i.i4.i.i.i56.i.i:                           ; preds = %bb11.i.i
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %_50.i.i, i64 noundef %13, i64 noundef range(i64 1, -9223372036854775807) 1) #77, !noalias !107862
  br label %bb33.i.i

bb8.i.i:                                          ; preds = %bb2.i.i.i4.i.i6.i.i.i.i, %bb4.i.i.i.i, %bb57.i.i
  %153 = icmp eq i64 %_23.i.sroa.0.0.copyload.i, -1
  br i1 %153, label %bb9.i.i, label %bb2.i60.i.i

bb2.i60.i.i:                                      ; preds = %bb8.i.i
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %_23.i.sroa.5.0.copyload.i) ]
  tail call void @llvm.experimental.noalias.scope.decl(metadata !107867)
  %_710.i.i.i.i.i.i = icmp eq i64 %_23.i.sroa.6.0.copyload.i, 0
  br i1 %_710.i.i.i.i.i.i, label %bb4.i.i69.i.i, label %bb5.i.i.i.i63.i.i

bb5.i.i.i.i63.i.i:                                ; preds = %bb2.i60.i.i, %core::ptr::drop_glue::<alloc::string::String> (.exit.i.i.i.i.i.i)
  %_3.sroa.0.011.i.i.i.i.i.i = phi i64 [ %154, %core::ptr::drop_glue::<alloc::string::String> (.exit.i.i.i.i.i.i) ], [ 0, %bb2.i60.i.i ]
  %_6.i.i.i.i64.i.i = getelementptr inbounds nuw [24 x i8], ptr %_23.i.sroa.5.0.copyload.i, i64 %_3.sroa.0.011.i.i.i.i.i.i
  %154 = add nuw nsw i64 %_3.sroa.0.011.i.i.i.i.i.i, 1
  tail call void @llvm.experimental.noalias.scope.decl(metadata !107870)
  %_1.val.i.i.i.i.i65.i.i = load i64, ptr %_6.i.i.i.i64.i.i, align 8, !alias.scope !107873, !noalias !107874
  %155 = icmp eq i64 %_1.val.i.i.i.i.i65.i.i, 0
  br i1 %155, label %core::ptr::drop_glue::<alloc::string::String> (.exit.i.i.i.i.i.i), label %bb2.i.i.i4.i.i.i.i.i.i66.i.i

bb2.i.i.i4.i.i.i.i.i.i66.i.i:                     ; preds = %bb5.i.i.i.i63.i.i
  %156 = getelementptr inbounds nuw i8, ptr %_6.i.i.i.i64.i.i, i64 8
  %_1.val1.i.i.i.i.i67.i.i = load ptr, ptr %156, align 8, !alias.scope !107873, !noalias !107874, !nonnull !3892, !noundef !3892
; call __rustc::__rust_dealloc
  tail call void @__rustc::__rust_dealloc(ptr noundef nonnull %_1.val1.i.i.i.i.i67.i.i, i64 noundef %_1.val.i.i.i.i.i65.i.i, i64 noundef range(i64 1, -9223372036854775807) 1) #77, !noalias !107879
  br label %core::ptr::drop_glue::<alloc::string::String> (.exit.i.i.i.i.i.i)

core::ptr::drop_glue::<alloc::string::String> (.exit.i.i.i.i.i.i): ; preds = %bb2.i.i.i4.i.i.i.i.i.i66.i.i, %bb5.i.i.i.i63.i.i
  %_7.i.i.i.i68.i.i = icmp eq i64 %154, %_23.i.sroa.6.0.copyload.i
  br i1 %_7.i.i.i.i68.i.i, label %bb4.i.i69.i.i, label %bb5.i.i.i.i63.i.i

bb4.i.i69.i.i:                                    ; preds = %core::ptr::drop_glue::<alloc::string::String> (.exit.i.i.i.i.i.i), %bb2.i60.i.i
  %157 = icmp eq i64 %_23.i.sroa.0.0.copyload.i, 0
  br i1 %157, label %bb9.i.i, label %bb2.i.i.i6.i.i70.i.i

bb2.i.i.i6.i.i70.i.i:                             ; preds = %bb4.i.i69.i.i
  %alloc_size.i.i.i.i7.i.i71.i.i = mul nuw i64 %_23.i.sroa.0.0.copyload.i, 24
; call __rustc::__rust_dealloc
  tail call void @__rustc::__rust_dealloc(ptr noundef nonnull %_23.i.sroa.5.0.copyload.i, i64 noundef %alloc_size.i.i.i.i7.i.i71.i.i, i64 noundef range(i64 1, -9223372036854775807) 8) #77, !noalias !107874
  br label %bb9.i.i

bb9.i.i:                                          ; preds = %bb2.i.i.i6.i.i70.i.i, %bb4.i.i69.i.i, %bb8.i.i
  %158 = icmp eq i64 %_22.i.sroa.0.0.copyload.i, -1
  br i1 %158, label %bb10.i.i, label %bb2.i73.i.i

bb2.i73.i.i:                                      ; preds = %bb9.i.i
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %_22.i.sroa.5.0.copyload.i) ]
  tail call void @llvm.experimental.noalias.scope.decl(metadata !107880)
  %_710.i.i.i.i76.i.i = icmp eq i64 %_22.i.sroa.6.0.copyload.i, 0
  br i1 %_710.i.i.i.i76.i.i, label %bb4.i.i85.i.i, label %bb5.i.i.i.i77.i.i

bb5.i.i.i.i77.i.i:                                ; preds = %bb2.i73.i.i, %core::ptr::drop_glue::<alloc::string::String> (.exit.i.i.i.i83.i.i)
  %_3.sroa.0.011.i.i.i.i78.i.i = phi i64 [ %159, %core::ptr::drop_glue::<alloc::string::String> (.exit.i.i.i.i83.i.i) ], [ 0, %bb2.i73.i.i ]
  %_6.i.i.i.i79.i.i = getelementptr inbounds nuw [24 x i8], ptr %_22.i.sroa.5.0.copyload.i, i64 %_3.sroa.0.011.i.i.i.i78.i.i
  %159 = add nuw nsw i64 %_3.sroa.0.011.i.i.i.i78.i.i, 1
  tail call void @llvm.experimental.noalias.scope.decl(metadata !107883)
  %_1.val.i.i.i.i.i80.i.i = load i64, ptr %_6.i.i.i.i79.i.i, align 8, !alias.scope !107886, !noalias !107887
  %160 = icmp eq i64 %_1.val.i.i.i.i.i80.i.i, 0
  br i1 %160, label %core::ptr::drop_glue::<alloc::string::String> (.exit.i.i.i.i83.i.i), label %bb2.i.i.i4.i.i.i.i.i.i81.i.i

bb2.i.i.i4.i.i.i.i.i.i81.i.i:                     ; preds = %bb5.i.i.i.i77.i.i
  %161 = getelementptr inbounds nuw i8, ptr %_6.i.i.i.i79.i.i, i64 8
  %_1.val1.i.i.i.i.i82.i.i = load ptr, ptr %161, align 8, !alias.scope !107886, !noalias !107887, !nonnull !3892, !noundef !3892
; call __rustc::__rust_dealloc
  tail call void @__rustc::__rust_dealloc(ptr noundef nonnull %_1.val1.i.i.i.i.i82.i.i, i64 noundef %_1.val.i.i.i.i.i80.i.i, i64 noundef range(i64 1, -9223372036854775807) 1) #77, !noalias !107892
  br label %core::ptr::drop_glue::<alloc::string::String> (.exit.i.i.i.i83.i.i)

core::ptr::drop_glue::<alloc::string::String> (.exit.i.i.i.i83.i.i): ; preds = %bb2.i.i.i4.i.i.i.i.i.i81.i.i, %bb5.i.i.i.i77.i.i
  %_7.i.i.i.i84.i.i = icmp eq i64 %159, %_22.i.sroa.6.0.copyload.i
  br i1 %_7.i.i.i.i84.i.i, label %bb4.i.i85.i.i, label %bb5.i.i.i.i77.i.i

bb4.i.i85.i.i:                                    ; preds = %core::ptr::drop_glue::<alloc::string::String> (.exit.i.i.i.i83.i.i), %bb2.i73.i.i
  %162 = icmp eq i64 %_22.i.sroa.0.0.copyload.i, 0
  br i1 %162, label %bb10.i.i, label %bb2.i.i.i6.i.i86.i.i

bb2.i.i.i6.i.i86.i.i:                             ; preds = %bb4.i.i85.i.i
  %alloc_size.i.i.i.i7.i.i87.i.i = mul nuw i64 %_22.i.sroa.0.0.copyload.i, 24
; call __rustc::__rust_dealloc
  tail call void @__rustc::__rust_dealloc(ptr noundef nonnull %_22.i.sroa.5.0.copyload.i, i64 noundef %alloc_size.i.i.i.i7.i.i87.i.i, i64 noundef range(i64 1, -9223372036854775807) 8) #77, !noalias !107887
  br label %bb10.i.i

bb10.i.i:                                         ; preds = %bb2.i.i.i6.i.i86.i.i, %bb4.i.i85.i.i, %bb9.i.i
  call void @llvm.lifetime.end.p0(ptr nonnull %config.sroa.0.i.i)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !107893)
  %163 = getelementptr inbounds nuw i8, ptr %specs.i.i, i64 8
  %_1.val.i.i.i = load ptr, ptr %163, align 8, !alias.scope !107893, !noalias !107652, !nonnull !3892, !noundef !3892
  %164 = getelementptr inbounds nuw i8, ptr %specs.i.i, i64 16
  %_1.val1.i.i.i = load i64, ptr %164, align 8, !alias.scope !107893, !noalias !107652, !noundef !3892
  %_7.i.i.i.i.i28 = icmp eq i64 %_1.val1.i.i.i, 0
  br i1 %_7.i.i.i.i.i28, label %bb4.i.i.i, label %bb5.i.i.i90.i.i

bb6.i.i.i.i.i:                                    ; preds = %bb5.i.i.i90.i.i
  %_7.i.i.i.i.i = icmp eq i64 %165, %_1.val1.i.i.i
  br i1 %_7.i.i.i.i.i, label %bb4.i.i.i, label %bb5.i.i.i90.i.i

bb5.i.i.i90.i.i:                                  ; preds = %bb10.i.i, %bb6.i.i.i.i.i
  %_3.sroa.0.0.i.i.i.i.i29 = phi i64 [ %165, %bb6.i.i.i.i.i ], [ 0, %bb10.i.i ]
  %_6.i.i.i.i.i = getelementptr inbounds nuw [160 x i8], ptr %_1.val.i.i.i, i64 %_3.sroa.0.0.i.i.i.i.i29
  %165 = add nuw nsw i64 %_3.sroa.0.0.i.i.i.i.i29, 1
; invoke core::ptr::drop_glue::<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>
  invoke fastcc void @core::ptr::drop_glue::<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>(ptr noalias nofree noundef align 8 dereferenceable(160) %_6.i.i.i.i.i)
          to label %bb6.i.i.i.i.i unwind label %cleanup.i.i.i91.i.i, !noalias !107896

bb4.i.i.i92.i.i:                                  ; preds = %bb3.i.i.i.i.i
  %166 = add i64 %_3.sroa.0.1.i.i.i.i.i31, 1
  %_5.i.i.i.i.i = icmp eq i64 %166, %_1.val1.i.i.i
  br i1 %_5.i.i.i.i.i, label %cleanup.body.i.i.i, label %bb3.i.i.i.i.i

cleanup.i.i.i91.i.i:                              ; preds = %bb5.i.i.i90.i.i
  %167 = landingpad { ptr, i32 }
          cleanup
  %_5.i.i.i.i.i30 = icmp eq i64 %165, %_1.val1.i.i.i
  br i1 %_5.i.i.i.i.i30, label %cleanup.body.i.i.i, label %bb3.i.i.i.i.i

bb3.i.i.i.i.i:                                    ; preds = %cleanup.i.i.i91.i.i, %bb4.i.i.i92.i.i
  %_3.sroa.0.1.i.i.i.i.i31 = phi i64 [ %166, %bb4.i.i.i92.i.i ], [ %165, %cleanup.i.i.i91.i.i ]
  %_4.i.i.i.i.i = getelementptr inbounds nuw [160 x i8], ptr %_1.val.i.i.i, i64 %_3.sroa.0.1.i.i.i.i.i31
; invoke core::ptr::drop_glue::<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>
  invoke fastcc void @core::ptr::drop_glue::<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>(ptr noalias nofree noundef align 8 dereferenceable(160) %_4.i.i.i.i.i) #79
          to label %bb4.i.i.i92.i.i unwind label %terminate.i.i.i93.i.i, !noalias !107896

terminate.i.i.i93.i.i:                            ; preds = %bb3.i.i.i.i.i
  %168 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  tail call void @core::panicking::panic_in_cleanup() #75, !noalias !107896
  unreachable

cleanup.body.i.i.i:                               ; preds = %bb4.i.i.i92.i.i, %cleanup.i.i.i91.i.i
  %_1.val2.i.i.i = load i64, ptr %specs.i.i, align 8, !alias.scope !107893, !noalias !107652
  %169 = icmp eq i64 %_1.val2.i.i.i, 0
  br i1 %169, label %bb37.i.i, label %bb2.i.i.i.i94.i.i

bb2.i.i.i.i94.i.i:                                ; preds = %cleanup.body.i.i.i
  %alloc_size.i.i.i.i.i.i.i = mul nuw i64 %_1.val2.i.i.i, 160
; call __rustc::__rust_dealloc
  tail call void @__rustc::__rust_dealloc(ptr noundef nonnull %_1.val.i.i.i, i64 noundef %alloc_size.i.i.i.i.i.i.i, i64 noundef range(i64 1, -9223372036854775807) 8) #77, !noalias !107896
  br label %bb37.i.i

bb4.i.i.i:                                        ; preds = %bb6.i.i.i.i.i, %bb10.i.i
  %_1.val4.i.i.i = load i64, ptr %specs.i.i, align 8, !alias.scope !107893, !noalias !107652
  %170 = icmp eq i64 %_1.val4.i.i.i, 0
  br i1 %170, label %bb62.i.i, label %bb2.i.i.i6.i.i.i

bb2.i.i.i6.i.i.i:                                 ; preds = %bb4.i.i.i
  %alloc_size.i.i.i.i7.i.i.i = mul nuw i64 %_1.val4.i.i.i, 160
; call __rustc::__rust_dealloc
  tail call void @__rustc::__rust_dealloc(ptr noundef nonnull %_1.val.i.i.i, i64 noundef %alloc_size.i.i.i.i7.i.i.i, i64 noundef range(i64 1, -9223372036854775807) 8) #77, !noalias !107896
  br label %bb62.i.i

bb62.i.i:                                         ; preds = %bb2.i.i.i6.i.i.i, %bb4.i.i.i, %bb55.i.i
  %_42.sroa.0.5.i.i = phi i8 [ 1, %bb55.i.i ], [ 0, %bb4.i.i.i ], [ 0, %bb2.i.i.i6.i.i.i ]
  call void @llvm.lifetime.end.p0(ptr nonnull %specs.i.i), !noalias !107652
; invoke core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_core::ir::term::TermValue)>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_core::ir::term::TermValue)>>(ptr noalias nofree noundef align 8 dereferenceable(24) %subs.i.i)
          to label %bb11.i.i unwind label %cleanup.i.i, !noalias !107652

terminate.i.i:                                    ; preds = %bb37.i.i, %cleanup8.i.i
  %171 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  tail call void @core::panicking::panic_in_cleanup() #75, !noalias !107652
  unreachable

bb33.i.i:                                         ; preds = %bb2.i.i.i4.i.i.i56.i.i, %bb11.i.i
  %172 = trunc nuw i8 %_42.sroa.0.4.i.i to i1
  br i1 %172, label %bb34.i.i, label %bb27.i.i

bb34.i.i:                                         ; preds = %bb2.i.i.i4.i.i.i115.i.i, %bb61.i.i, %bb33.i.i
  %_39.sroa.0.7.i.i = phi i1 [ %_39.sroa.0.4.i.i, %bb33.i.i ], [ true, %bb61.i.i ], [ true, %bb2.i.i.i4.i.i.i115.i.i ]
  call void @llvm.experimental.noalias.scope.decl(metadata !107897)
  %173 = load i64, ptr %4, align 8, !range !3909, !alias.scope !107900, !noalias !107654, !noundef !3892
  %174 = icmp eq i64 %173, -1
  br i1 %174, label %bb32.i.i, label %bb2.i97.i.i

bb2.i97.i.i:                                      ; preds = %bb34.i.i
  call void @llvm.experimental.noalias.scope.decl(metadata !107901)
  %175 = getelementptr inbounds nuw i8, ptr %_14, i64 32
  %_1.val.i.i98.i.i = load ptr, ptr %175, align 8, !alias.scope !107904, !noalias !107654, !nonnull !3892, !noundef !3892
  %176 = getelementptr inbounds nuw i8, ptr %_14, i64 40
  %_1.val1.i.i99.i.i = load i64, ptr %176, align 8, !alias.scope !107904, !noalias !107654, !noundef !3892
  call void @llvm.experimental.noalias.scope.decl(metadata !107905)
  %_710.i.i.i.i100.i.i = icmp eq i64 %_1.val1.i.i99.i.i, 0
  br i1 %_710.i.i.i.i100.i.i, label %bb4.i.i109.i.i, label %bb5.i.i.i.i101.i.i

bb5.i.i.i.i101.i.i:                               ; preds = %bb2.i97.i.i, %core::ptr::drop_glue::<alloc::string::String> (.exit.i.i.i.i107.i.i)
  %_3.sroa.0.011.i.i.i.i102.i.i = phi i64 [ %177, %core::ptr::drop_glue::<alloc::string::String> (.exit.i.i.i.i107.i.i) ], [ 0, %bb2.i97.i.i ]
  %_6.i.i.i.i103.i.i = getelementptr inbounds nuw [24 x i8], ptr %_1.val.i.i98.i.i, i64 %_3.sroa.0.011.i.i.i.i102.i.i
  %177 = add nuw nsw i64 %_3.sroa.0.011.i.i.i.i102.i.i, 1
  call void @llvm.experimental.noalias.scope.decl(metadata !107908)
  %_1.val.i.i.i.i.i104.i.i = load i64, ptr %_6.i.i.i.i103.i.i, align 8, !alias.scope !107911, !noalias !107912
  %178 = icmp eq i64 %_1.val.i.i.i.i.i104.i.i, 0
  br i1 %178, label %core::ptr::drop_glue::<alloc::string::String> (.exit.i.i.i.i107.i.i), label %bb2.i.i.i4.i.i.i.i.i.i105.i.i

bb2.i.i.i4.i.i.i.i.i.i105.i.i:                    ; preds = %bb5.i.i.i.i101.i.i
  %179 = getelementptr inbounds nuw i8, ptr %_6.i.i.i.i103.i.i, i64 8
  %_1.val1.i.i.i.i.i106.i.i = load ptr, ptr %179, align 8, !alias.scope !107911, !noalias !107912, !nonnull !3892, !noundef !3892
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %_1.val1.i.i.i.i.i106.i.i, i64 noundef %_1.val.i.i.i.i.i104.i.i, i64 noundef range(i64 1, -9223372036854775807) 1) #77, !noalias !107913
  br label %core::ptr::drop_glue::<alloc::string::String> (.exit.i.i.i.i107.i.i)

core::ptr::drop_glue::<alloc::string::String> (.exit.i.i.i.i107.i.i): ; preds = %bb2.i.i.i4.i.i.i.i.i.i105.i.i, %bb5.i.i.i.i101.i.i
  %_7.i.i.i.i108.i.i = icmp eq i64 %177, %_1.val1.i.i99.i.i
  br i1 %_7.i.i.i.i108.i.i, label %bb4.i.i109.i.i, label %bb5.i.i.i.i101.i.i

bb4.i.i109.i.i:                                   ; preds = %core::ptr::drop_glue::<alloc::string::String> (.exit.i.i.i.i107.i.i), %bb2.i97.i.i
  %180 = icmp eq i64 %173, 0
  br i1 %180, label %bb32.i.i, label %bb2.i.i.i6.i.i110.i.i

bb2.i.i.i6.i.i110.i.i:                            ; preds = %bb4.i.i109.i.i
  %alloc_size.i.i.i.i7.i.i111.i.i = mul nuw i64 %173, 24
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %_1.val.i.i98.i.i, i64 noundef %alloc_size.i.i.i.i7.i.i111.i.i, i64 noundef range(i64 1, -9223372036854775807) 8) #77, !noalias !107912
  br label %bb32.i.i

bb37.i.i:                                         ; preds = %bb2.i.i.i.i94.i.i, %cleanup.body.i.i.i, %cleanup8.i.i, %cleanup7.i.i
  %_42.sroa.0.2.ph.i.i = phi i8 [ 0, %cleanup8.i.i ], [ 1, %cleanup7.i.i ], [ 0, %bb2.i.i.i.i94.i.i ], [ 0, %cleanup.body.i.i.i ]
  %.pn33.ph.i.i = phi { ptr, i32 } [ %30, %cleanup8.i.i ], [ %26, %cleanup7.i.i ], [ %167, %bb2.i.i.i.i94.i.i ], [ %167, %cleanup.body.i.i.i ]
; invoke core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_core::ir::term::TermValue)>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_core::ir::term::TermValue)>>(ptr noalias nofree noundef align 8 dereferenceable(24) %subs.i.i) #79
          to label %bb47.i.i unwind label %terminate.i.i, !noalias !107652

bb61.i.i:                                         ; preds = %bb53.i.i, %bb51.i.i
  %181 = icmp sgt i64 %13, 0
  br i1 %181, label %bb2.i.i.i4.i.i.i115.i.i, label %bb34.i.i

bb2.i.i.i4.i.i.i115.i.i:                          ; preds = %bb61.i.i
; call __rustc::__rust_dealloc
  tail call void @__rustc::__rust_dealloc(ptr noundef nonnull %_50.i.i, i64 noundef %13, i64 noundef range(i64 1, -9223372036854775807) 1) #77, !noalias !107914
  br label %bb34.i.i

bb32.i.i:                                         ; preds = %bb2.i.i.i6.i.i110.i.i, %bb4.i.i109.i.i, %bb34.i.i
  call void @llvm.experimental.noalias.scope.decl(metadata !107919)
  %182 = load i64, ptr %5, align 8, !range !3909, !alias.scope !107922, !noalias !107654, !noundef !3892
  %183 = icmp eq i64 %182, -1
  br i1 %183, label %bb30.i.i, label %bb2.i119.i.i

bb2.i119.i.i:                                     ; preds = %bb32.i.i
  call void @llvm.experimental.noalias.scope.decl(metadata !107923)
  %184 = getelementptr inbounds nuw i8, ptr %_14, i64 56
  %_1.val.i.i120.i.i = load ptr, ptr %184, align 8, !alias.scope !107926, !noalias !107654, !nonnull !3892, !noundef !3892
  %185 = getelementptr inbounds nuw i8, ptr %_14, i64 64
  %_1.val1.i.i121.i.i = load i64, ptr %185, align 8, !alias.scope !107926, !noalias !107654, !noundef !3892
  call void @llvm.experimental.noalias.scope.decl(metadata !107927)
  %_710.i.i.i.i122.i.i = icmp eq i64 %_1.val1.i.i121.i.i, 0
  br i1 %_710.i.i.i.i122.i.i, label %bb4.i.i131.i.i, label %bb5.i.i.i.i123.i.i

bb5.i.i.i.i123.i.i:                               ; preds = %bb2.i119.i.i, %core::ptr::drop_glue::<alloc::string::String> (.exit.i.i.i.i129.i.i)
  %_3.sroa.0.011.i.i.i.i124.i.i = phi i64 [ %186, %core::ptr::drop_glue::<alloc::string::String> (.exit.i.i.i.i129.i.i) ], [ 0, %bb2.i119.i.i ]
  %_6.i.i.i.i125.i.i = getelementptr inbounds nuw [24 x i8], ptr %_1.val.i.i120.i.i, i64 %_3.sroa.0.011.i.i.i.i124.i.i
  %186 = add nuw nsw i64 %_3.sroa.0.011.i.i.i.i124.i.i, 1
  call void @llvm.experimental.noalias.scope.decl(metadata !107930)
  %_1.val.i.i.i.i.i126.i.i = load i64, ptr %_6.i.i.i.i125.i.i, align 8, !alias.scope !107933, !noalias !107934
  %187 = icmp eq i64 %_1.val.i.i.i.i.i126.i.i, 0
  br i1 %187, label %core::ptr::drop_glue::<alloc::string::String> (.exit.i.i.i.i129.i.i), label %bb2.i.i.i4.i.i.i.i.i.i127.i.i

bb2.i.i.i4.i.i.i.i.i.i127.i.i:                    ; preds = %bb5.i.i.i.i123.i.i
  %188 = getelementptr inbounds nuw i8, ptr %_6.i.i.i.i125.i.i, i64 8
  %_1.val1.i.i.i.i.i128.i.i = load ptr, ptr %188, align 8, !alias.scope !107933, !noalias !107934, !nonnull !3892, !noundef !3892
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %_1.val1.i.i.i.i.i128.i.i, i64 noundef %_1.val.i.i.i.i.i126.i.i, i64 noundef range(i64 1, -9223372036854775807) 1) #77, !noalias !107935
  br label %core::ptr::drop_glue::<alloc::string::String> (.exit.i.i.i.i129.i.i)

core::ptr::drop_glue::<alloc::string::String> (.exit.i.i.i.i129.i.i): ; preds = %bb2.i.i.i4.i.i.i.i.i.i127.i.i, %bb5.i.i.i.i123.i.i
  %_7.i.i.i.i130.i.i = icmp eq i64 %186, %_1.val1.i.i121.i.i
  br i1 %_7.i.i.i.i130.i.i, label %bb4.i.i131.i.i, label %bb5.i.i.i.i123.i.i

bb4.i.i131.i.i:                                   ; preds = %core::ptr::drop_glue::<alloc::string::String> (.exit.i.i.i.i129.i.i), %bb2.i119.i.i
  %189 = icmp eq i64 %182, 0
  br i1 %189, label %bb30.i.i, label %bb2.i.i.i6.i.i132.i.i

bb2.i.i.i6.i.i132.i.i:                            ; preds = %bb4.i.i131.i.i
  %alloc_size.i.i.i.i7.i.i133.i.i = mul nuw i64 %182, 24
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %_1.val.i.i120.i.i, i64 noundef %alloc_size.i.i.i.i7.i.i133.i.i, i64 noundef range(i64 1, -9223372036854775807) 8) #77, !noalias !107934
  br label %bb30.i.i

bb27.i.i:                                         ; preds = %bb4.i.i139.i.i, %bb30.i.i, %bb33.i.i
  %_39.sroa.0.6229.i.i = phi i1 [ %_39.sroa.0.7.i.i, %bb30.i.i ], [ %_39.sroa.0.7.i.i, %bb4.i.i139.i.i ], [ %_39.sroa.0.4.i.i, %bb33.i.i ]
  br i1 %_39.sroa.0.6229.i.i, label %bb28.i.i, label %<purrdf_native::py_store::quad_store::PyQuadStore>::query::{closure#0} (.exit.i)

bb30.i.i:                                         ; preds = %bb2.i.i.i6.i.i132.i.i, %bb4.i.i131.i.i, %bb32.i.i
  call void @llvm.experimental.noalias.scope.decl(metadata !107936)
  %190 = load i64, ptr %6, align 8, !range !3909, !alias.scope !107939, !noalias !107654, !noundef !3892
  %191 = icmp eq i64 %190, -1
  br i1 %191, label %bb27.i.i, label %bb2.i136.i.i

bb2.i136.i.i:                                     ; preds = %bb30.i.i
  call void @llvm.experimental.noalias.scope.decl(metadata !107940)
  call void @llvm.experimental.noalias.scope.decl(metadata !107943)
  %192 = icmp eq i64 %190, 0
  br i1 %192, label %bb4.i.i139.i.i, label %bb2.i.i.i4.i.i.i.i137.i.i

bb2.i.i.i4.i.i.i.i137.i.i:                        ; preds = %bb2.i136.i.i
  %193 = getelementptr inbounds nuw i8, ptr %_14, i64 80
  %_1.val1.i.i.i138.i.i = load ptr, ptr %193, align 8, !alias.scope !107946, !noalias !107654, !nonnull !3892, !noundef !3892
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %_1.val1.i.i.i138.i.i, i64 noundef %190, i64 noundef range(i64 1, -9223372036854775807) 1) #77, !noalias !107947
  br label %bb4.i.i139.i.i

bb4.i.i139.i.i:                                   ; preds = %bb2.i.i.i4.i.i.i.i137.i.i, %bb2.i136.i.i
  %194 = getelementptr inbounds nuw i8, ptr %_14, i64 96
  call void @llvm.experimental.noalias.scope.decl(metadata !107948)
  %_1.val.i5.i.i140.i.i = load i64, ptr %194, align 8, !alias.scope !107951, !noalias !107654
  %195 = icmp eq i64 %_1.val.i5.i.i140.i.i, 0
  br i1 %195, label %bb27.i.i, label %bb2.i.i.i4.i.i6.i.i141.i.i

bb2.i.i.i4.i.i6.i.i141.i.i:                       ; preds = %bb4.i.i139.i.i
  %196 = getelementptr inbounds nuw i8, ptr %_14, i64 104
  %_1.val1.i7.i.i142.i.i = load ptr, ptr %196, align 8, !alias.scope !107951, !noalias !107654, !nonnull !3892, !noundef !3892
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %_1.val1.i7.i.i142.i.i, i64 noundef %_1.val.i5.i.i140.i.i, i64 noundef range(i64 1, -9223372036854775807) 1) #77, !noalias !107952
  br i1 %_39.sroa.0.7.i.i, label %bb28.i.i, label %<purrdf_native::py_store::quad_store::PyQuadStore>::query::{closure#0} (.exit.i)

bb28.i.i:                                         ; preds = %bb2.i.i.i4.i.i6.i.i141.i.i, %bb27.i.i
  call void @llvm.experimental.noalias.scope.decl(metadata !107953)
  %197 = load i64, ptr %10, align 8, !range !3909, !alias.scope !107956, !noalias !107654, !noundef !3892
  %198 = icmp eq i64 %197, -1
  br i1 %198, label %<purrdf_native::py_store::quad_store::PyQuadStore>::query::{closure#0} (.exit.i), label %bb2.i145.i.i

bb2.i145.i.i:                                     ; preds = %bb28.i.i
  call void @llvm.experimental.noalias.scope.decl(metadata !107957)
  %199 = icmp eq i64 %197, 0
  br i1 %199, label %<purrdf_native::py_store::quad_store::PyQuadStore>::query::{closure#0} (.exit.i), label %bb2.i.i.i4.i.i.i146.i.i

bb2.i.i.i4.i.i.i146.i.i:                          ; preds = %bb2.i145.i.i
  %200 = getelementptr inbounds nuw i8, ptr %_14, i64 128
  %_1.val1.i.i147.i.i = load ptr, ptr %200, align 8, !alias.scope !107960, !noalias !107654, !nonnull !3892, !noundef !3892
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %_1.val1.i.i147.i.i, i64 noundef %197, i64 noundef range(i64 1, -9223372036854775807) 1) #77, !noalias !107961
  br label %<purrdf_native::py_store::quad_store::PyQuadStore>::query::{closure#0} (.exit.i)

common.resume.i:                                  ; preds = %bb6.i2.i, %cleanup1.body.i.i, %bb2.i.i.i4.i.i.i156.i.i, %bb2.i155.i.i, %bb40.i.i, %bb39.i.i
  %common.resume.op.i = phi { ptr, i32 } [ %.pn35.i.i, %bb39.i.i ], [ %.pn35.i.i, %bb2.i.i.i4.i.i.i156.i.i ], [ %.pn35.i.i, %bb2.i155.i.i ], [ %.pn35.i.i, %bb40.i.i ], [ %239, %bb6.i2.i ], [ %eh.lpad-body.i.i, %cleanup1.body.i.i ]
  resume { ptr, i32 } %common.resume.op.i

bb45.i.i:                                         ; preds = %bb2.i.i.i4.i.i.i.i.i, %bb47.i.i
  %cond44.i.i = icmp eq i8 %_42.sroa.0.0.i.i, 0
  br i1 %cond44.i.i, label %bb39.i.i, label %bb46.i.i

bb46.i.i:                                         ; preds = %bb45.i.i
; call core::ptr::drop_glue::<core::option::Option<alloc::vec::Vec<alloc::string::String>>>
  call fastcc void @core::ptr::drop_glue::<core::option::Option<alloc::vec::Vec<alloc::string::String>>>(ptr noalias nofree noundef readonly align 8 dereferenceable(24) %4) #79, !noalias !107654
; call core::ptr::drop_glue::<core::option::Option<alloc::vec::Vec<alloc::string::String>>>
  call fastcc void @core::ptr::drop_glue::<core::option::Option<alloc::vec::Vec<alloc::string::String>>>(ptr noalias nofree noundef readonly align 8 dereferenceable(24) %5) #79, !noalias !107654
; call core::ptr::drop_glue::<core::option::Option<purrdf_sparql_results::model::ProvenanceNamespace>>
  call fastcc void @core::ptr::drop_glue::<core::option::Option<purrdf_sparql_results::model::ProvenanceNamespace>>(ptr noalias nofree noundef readonly align 8 dereferenceable(48) %6) #79, !noalias !107654
  br label %bb39.i.i

bb39.i.i:                                         ; preds = %bb46.i.i, %bb45.i.i
  br i1 %_39.sroa.0.0.i.i, label %bb40.i.i, label %common.resume.i

bb40.i.i:                                         ; preds = %bb39.i.i
  call void @llvm.experimental.noalias.scope.decl(metadata !107962)
  %201 = load i64, ptr %10, align 8, !range !3909, !alias.scope !107965, !noalias !107654, !noundef !3892
  %202 = icmp eq i64 %201, -1
  br i1 %202, label %common.resume.i, label %bb2.i155.i.i

bb2.i155.i.i:                                     ; preds = %bb40.i.i
  call void @llvm.experimental.noalias.scope.decl(metadata !107966)
  %203 = icmp eq i64 %201, 0
  br i1 %203, label %common.resume.i, label %bb2.i.i.i4.i.i.i156.i.i

bb2.i.i.i4.i.i.i156.i.i:                          ; preds = %bb2.i155.i.i
  %204 = getelementptr inbounds nuw i8, ptr %_14, i64 128
  %_1.val1.i.i157.i.i = load ptr, ptr %204, align 8, !alias.scope !107969, !noalias !107654, !nonnull !3892, !noundef !3892
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %_1.val1.i.i157.i.i, i64 noundef %201, i64 noundef range(i64 1, -9223372036854775807) 1) #77, !noalias !107970
  br label %common.resume.i

<purrdf_native::py_store::quad_store::PyQuadStore>::query::{closure#0} (.exit.i): ; preds = %bb2.i.i.i4.i.i.i146.i.i, %bb2.i145.i.i, %bb28.i.i, %bb2.i.i.i4.i.i6.i.i141.i.i, %bb27.i.i, %bb2.i.i.i4.i.i.i51.i.i, %bb7.i.i
  call void @llvm.lifetime.end.p0(ptr nonnull %_26.i.i), !noalias !107651
  call void @llvm.lifetime.end.p0(ptr nonnull %result.i.i), !noalias !107651
  %_2.i = load i64, ptr %_0, align 8, !range !6076, !alias.scope !107641, !noalias !107644, !noundef !3892
  %205 = trunc nuw i64 %_2.i to i1
  br i1 %205, label %bb4.i, label %purrdf_native::py_store::presentation::settled::<pyo3::instance::Py<pyo3::types::any::PyAny>, <purrdf_native::py_store::quad_store::PyQuadStore>::query::{closure#0}> (.exit)

bb4.i:                                            ; preds = %<purrdf_native::py_store::quad_store::PyQuadStore>::query::{closure#0} (.exit.i)
  %206 = getelementptr inbounds nuw i8, ptr %_0, i64 8
  call void @llvm.lifetime.start.p0(ptr nonnull %_4.sroa.0.i)
  call void @llvm.lifetime.start.p0(ptr nonnull %guard.i.i), !noalias !107971
; invoke <pyo3::internal::state::AttachGuard>::attach
  %207 = invoke noundef i32 @<pyo3::internal::state::AttachGuard>::attach()
          to label %bb1.i5.i unwind label %bb6.i2.i, !noalias !107971

bb1.i5.i:                                         ; preds = %bb4.i
  store i32 %207, ptr %guard.i.i, align 4, !noalias !107971
  call void @llvm.lifetime.start.p0(ptr nonnull %_3.i1.i), !noalias !107971
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %_3.i1.i, ptr noundef nonnull align 8 dereferenceable(48) %206, i64 48, i1 false), !noalias !107644
  call void @llvm.experimental.noalias.scope.decl(metadata !107975)
  call void @llvm.experimental.noalias.scope.decl(metadata !107978)
  %_2.i.i.i.i.i6.i = load ptr, ptr @PyExc_ValueError, align 8, !noalias !107980, !nonnull !3892, !noundef !3892
  call void @_Py_IncRef(ptr noundef nonnull %_2.i.i.i.i.i6.i) #77, !noalias !107980
; invoke <pyo3::err::PyErr>::get_type
  %208 = invoke noundef nonnull ptr @<pyo3::err::PyErr>::get_type(ptr noundef nonnull align 8 dereferenceable(48) %_3.i1.i)
          to label %bb1.i.i9.i unwind label %bb4.i.i.i7.i, !noalias !107981

bb4.i.i.i7.i:                                     ; preds = %bb1.i5.i
  %209 = landingpad { ptr, i32 }
          cleanup
  call void @_Py_DecRef(ptr noundef nonnull %_2.i.i.i.i.i6.i) #77, !noalias !107981
  br label %bb12.i.i.i

cleanup.i.i.i:                                    ; preds = %bb2.i.i.i
  %210 = landingpad { ptr, i32 }
          cleanup
  br label %bb12.i.i.i

bb1.i.i9.i:                                       ; preds = %bb1.i5.i
  %_8.i.i.i.i = call noundef i32 @PyErr_GivenExceptionMatches(ptr noundef nonnull %208, ptr noundef nonnull %_2.i.i.i.i.i6.i) #77, !noalias !107981
  call void @_Py_DecRef(ptr noundef nonnull %208) #77, !noalias !107981
  %_0.i.not.i.i.i = icmp eq i32 %_8.i.i.i.i, 0
  call void @_Py_DecRef(ptr noundef nonnull %_2.i.i.i.i.i6.i) #77, !noalias !107981
  br i1 %_0.i.not.i.i.i, label %bb3.i.i.i, label %bb2.i.i.i

bb3.i.i.i:                                        ; preds = %bb1.i.i9.i
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %_4.sroa.0.i, ptr noundef nonnull align 8 dereferenceable(16) %_3.i1.i, i64 16, i1 false), !alias.scope !107982, !noalias !107983
  %_4.sroa.6.0._3.i1.sroa_idx.i = getelementptr inbounds nuw i8, ptr %_3.i1.i, i64 16
  %_4.sroa.6.0.copyload18.i = load i64, ptr %_4.sroa.6.0._3.i1.sroa_idx.i, align 8, !alias.scope !107982, !noalias !107983
  %_4.sroa.7.0._3.i1.sroa_idx.i = getelementptr inbounds nuw i8, ptr %_3.i1.i, i64 24
  %211 = load <2 x ptr>, ptr %_4.sroa.7.0._3.i1.sroa_idx.i, align 8, !alias.scope !107982, !noalias !107983
  %_4.sroa.9.0._3.i1.sroa_idx.i = getelementptr inbounds nuw i8, ptr %_3.i1.i, i64 40
  %_4.sroa.9.0.copyload27.i = load i64, ptr %_4.sroa.9.0._3.i1.sroa_idx.i, align 8, !alias.scope !107982, !noalias !107983
  br label %<pyo3::marker::Python>::attach::<purrdf_native::py_store::presentation::settled<pyo3::instance::Py<pyo3::types::any::PyAny>, <purrdf_native::py_store::quad_store::PyQuadStore>::query::{closure#0}>::{closure#0}::{closure#0}, pyo3::err::PyErr> (.exit.i)

bb2.i.i.i:                                        ; preds = %bb1.i.i9.i
; invoke <pyo3::err::PyErr>::value
  %value.i.i.i = invoke noundef nonnull align 8 ptr @<pyo3::err::PyErr>::value(ptr noundef nonnull align 8 dereferenceable(48) %_3.i1.i)
          to label %bb2.lr.ph.i.i.i.i unwind label %cleanup.i.i.i, !noalias !107981

bb2.lr.ph.i.i.i.i:                                ; preds = %bb2.i.i.i
  %212 = getelementptr inbounds nuw i8, ptr %_4.i.i.i.i.i.i.i, i64 1
  %213 = getelementptr inbounds nuw i8, ptr %_4.i.i.i.i.i.i.i, i64 8
  %214 = getelementptr inbounds nuw i8, ptr %_9.i.i.i.i, i64 8
  call void @llvm.lifetime.start.p0(ptr nonnull %_9.i.i.i.i), !noalias !107984
  call void @llvm.lifetime.start.p0(ptr nonnull %_4.i.i.i.i.i.i.i), !noalias !107989
; invoke <pyo3::types::string::PyString>::new
  %_3.i.i.i.i.i.i4.i.i.i = invoke noundef nonnull ptr @<pyo3::types::string::PyString>::new(ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @alloc_016e71ba2f68cc7172262ae988bb360c, i64 noundef 10)
          to label %_3.i.i.i.i.i.i.noexc.i.i.i unwind label %cleanup1.i.i10.i, !noalias !107981

_3.i.i.i.i.i.i.noexc.i.i.i:                       ; preds = %bb2.lr.ph.i.i.i.i
; invoke <pyo3::instance::Bound<pyo3::types::any::PyAny> as pyo3::types::any::PyAnyMethods>::hasattr::inner
  invoke void @<pyo3::instance::Bound<pyo3::types::any::PyAny> as pyo3::types::any::PyAnyMethods>::hasattr::inner(ptr noalias nofree noundef nonnull sret([56 x i8]) align 8 captures(none) dereferenceable(56) %_4.i.i.i.i.i.i.i, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(8) %value.i.i.i, ptr noundef nonnull %_3.i.i.i.i.i.i4.i.i.i)
          to label %<pyo3::instance::Bound<pyo3::types::any::PyAny> as pyo3::types::any::PyAnyMethods>::hasattr::<&str> (.exit.i.i.i.i.i.i.i) unwind label %bb8.i.i.i.i.i.i.i.i, !noalias !107996

bb8.i.i.i.i.i.i.i.i:                              ; preds = %_3.i.i.i.i.i.i.noexc.1.i.i.i, %_3.i.i.i.i.i.i.noexc.i.i.i
  %_3.i.i.i.i.i.i4.lcssa.i.i.i = phi ptr [ %_3.i.i.i.i.i.i4.i.i.i, %_3.i.i.i.i.i.i.noexc.i.i.i ], [ %_3.i.i.i.i.i.i4.1.i.i.i, %_3.i.i.i.i.i.i.noexc.1.i.i.i ]
  %215 = landingpad { ptr, i32 }
          cleanup
  call void @_Py_DecRef(ptr noundef nonnull %_3.i.i.i.i.i.i4.lcssa.i.i.i) #77, !noalias !107999
  br label %bb12.i.i.i

<pyo3::instance::Bound<pyo3::types::any::PyAny> as pyo3::types::any::PyAnyMethods>::hasattr::<&str> (.exit.i.i.i.i.i.i.i): ; preds = %_3.i.i.i.i.i.i.noexc.i.i.i
  call void @_Py_DecRef(ptr noundef nonnull %_3.i.i.i.i.i.i4.i.i.i) #77, !noalias !107999
  %216 = load i8, ptr %_4.i.i.i.i.i.i.i, align 8, !range !4704, !noalias !108002, !noundef !3892
  %217 = trunc nuw i8 %216 to i1
  br i1 %217, label %<core::array::iter::iter_inner::PolymorphicIter<[core::mem::maybe_uninit::MaybeUninit<&str>]>>::try_fold::<(), core::iter::traits::iterator::Iterator::try_for_each::call<&str, core::result::Result<(), pyo3::err::PyErr>, purrdf_native::py_store::presentation::settled<pyo3::instance::Py<pyo3::types::any::PyAny>, <purrdf_native::py_store::quad_store::PyQuadStore>::query::{closure#0}>::{closure#0}::{closure#0}::{closure#0}>::{closure#0}, core::result::Result<(), pyo3::err::PyErr>>::{closure#0} (.exit.thread.i.i.i.i), label %bb8.i.i.i.i.i.i.i

<core::array::iter::iter_inner::PolymorphicIter<[core::mem::maybe_uninit::MaybeUninit<&str>]>>::try_fold::<(), core::iter::traits::iterator::Iterator::try_for_each::call<&str, core::result::Result<(), pyo3::err::PyErr>, purrdf_native::py_store::presentation::settled<pyo3::instance::Py<pyo3::types::any::PyAny>, <purrdf_native::py_store::quad_store::PyQuadStore>::query::{closure#0}>::{closure#0}::{closure#0}::{closure#0}>::{closure#0}, core::result::Result<(), pyo3::err::PyErr>>::{closure#0} (.exit.thread.i.i.i.i): ; preds = %<pyo3::instance::Bound<pyo3::types::any::PyAny> as pyo3::types::any::PyAnyMethods>::hasattr::<&str> (.exit.i.i.i.i.1.i.i.i), %<pyo3::instance::Bound<pyo3::types::any::PyAny> as pyo3::types::any::PyAnyMethods>::hasattr::<&str> (.exit.i.i.i.i.i.i.i)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %214, ptr noundef nonnull align 8 dereferenceable(48) %213, i64 48, i1 false), !noalias !108004
  call void @llvm.lifetime.end.p0(ptr nonnull %_4.i.i.i.i.i.i.i), !noalias !107989
  br label %bb5.i.i.i12.i

bb8.i.i.i.i.i.i.i:                                ; preds = %<pyo3::instance::Bound<pyo3::types::any::PyAny> as pyo3::types::any::PyAnyMethods>::hasattr::<&str> (.exit.i.i.i.i.i.i.i)
  %218 = load i8, ptr %212, align 1, !range !4704, !noalias !108002, !noundef !3892
  %_16.i.i.i.i.i.i.i = trunc nuw i8 %218 to i1
  br i1 %_16.i.i.i.i.i.i.i, label %<core::array::iter::iter_inner::PolymorphicIter<[core::mem::maybe_uninit::MaybeUninit<&str>]>>::try_fold::<(), core::iter::traits::iterator::Iterator::try_for_each::call<&str, core::result::Result<(), pyo3::err::PyErr>, purrdf_native::py_store::presentation::settled<pyo3::instance::Py<pyo3::types::any::PyAny>, <purrdf_native::py_store::quad_store::PyQuadStore>::query::{closure#0}>::{closure#0}::{closure#0}::{closure#0}>::{closure#0}, core::result::Result<(), pyo3::err::PyErr>>::{closure#0} (.exit.thread11.i.i.i.i), label %<core::array::iter::iter_inner::PolymorphicIter<[core::mem::maybe_uninit::MaybeUninit<&str>]>>::try_fold::<(), core::iter::traits::iterator::Iterator::try_for_each::call<&str, core::result::Result<(), pyo3::err::PyErr>, purrdf_native::py_store::presentation::settled<pyo3::instance::Py<pyo3::types::any::PyAny>, <purrdf_native::py_store::quad_store::PyQuadStore>::query::{closure#0}>::{closure#0}::{closure#0}::{closure#0}>::{closure#0}, core::result::Result<(), pyo3::err::PyErr>>::{closure#0} (.exit.i.i.i.i)

<core::array::iter::iter_inner::PolymorphicIter<[core::mem::maybe_uninit::MaybeUninit<&str>]>>::try_fold::<(), core::iter::traits::iterator::Iterator::try_for_each::call<&str, core::result::Result<(), pyo3::err::PyErr>, purrdf_native::py_store::presentation::settled<pyo3::instance::Py<pyo3::types::any::PyAny>, <purrdf_native::py_store::quad_store::PyQuadStore>::query::{closure#0}>::{closure#0}::{closure#0}::{closure#0}>::{closure#0}, core::result::Result<(), pyo3::err::PyErr>>::{closure#0} (.exit.thread11.i.i.i.i): ; preds = %bb8.i.i.i.i.i.i.i
  call void @llvm.lifetime.end.p0(ptr nonnull %_4.i.i.i.i.i.i.i), !noalias !107989
  br label %bb6.i.i.i11.i

<core::array::iter::iter_inner::PolymorphicIter<[core::mem::maybe_uninit::MaybeUninit<&str>]>>::try_fold::<(), core::iter::traits::iterator::Iterator::try_for_each::call<&str, core::result::Result<(), pyo3::err::PyErr>, purrdf_native::py_store::presentation::settled<pyo3::instance::Py<pyo3::types::any::PyAny>, <purrdf_native::py_store::quad_store::PyQuadStore>::query::{closure#0}>::{closure#0}::{closure#0}::{closure#0}>::{closure#0}, core::result::Result<(), pyo3::err::PyErr>>::{closure#0} (.exit.i.i.i.i): ; preds = %bb8.i.i.i.i.i.i.i
  %_26.i.i.i.i.i.i.i = call noundef ptr @Py_GetConstantBorrowed(i32 noundef 0) #77, !noalias !107996
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %_26.i.i.i.i.i.i.i) ]
  call void @_Py_IncRef(ptr noundef nonnull %_26.i.i.i.i.i.i.i) #77, !noalias !107996
; invoke <pyo3::instance::Bound<pyo3::types::any::PyAny> as pyo3::types::any::PyAnyMethods>::setattr::<&str, pyo3::instance::Py<pyo3::types::any::PyAny>>
  invoke fastcc void @<pyo3::instance::Bound<pyo3::types::any::PyAny> as pyo3::types::any::PyAnyMethods>::setattr::<&str, pyo3::instance::Py<pyo3::types::any::PyAny>>(ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(56) %_9.i.i.i.i, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(8) %value.i.i.i, ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @alloc_016e71ba2f68cc7172262ae988bb360c, i64 noundef 10, ptr noundef nonnull %_26.i.i.i.i.i.i.i)
          to label %.noexc.i.i.i unwind label %cleanup1.i.i10.i, !noalias !107981

.noexc.i.i.i:                                     ; preds = %<core::array::iter::iter_inner::PolymorphicIter<[core::mem::maybe_uninit::MaybeUninit<&str>]>>::try_fold::<(), core::iter::traits::iterator::Iterator::try_for_each::call<&str, core::result::Result<(), pyo3::err::PyErr>, purrdf_native::py_store::presentation::settled<pyo3::instance::Py<pyo3::types::any::PyAny>, <purrdf_native::py_store::quad_store::PyQuadStore>::query::{closure#0}>::{closure#0}::{closure#0}::{closure#0}>::{closure#0}, core::result::Result<(), pyo3::err::PyErr>>::{closure#0} (.exit.i.i.i.i)
  %_2.i.pre.i.i.i.i = load i64, ptr %_9.i.i.i.i, align 8, !range !6076, !alias.scope !108005, !noalias !108008
  %219 = trunc nuw i64 %_2.i.pre.i.i.i.i to i1
  call void @llvm.lifetime.end.p0(ptr nonnull %_4.i.i.i.i.i.i.i), !noalias !107989
  br i1 %219, label %bb5.i.i.i12.i, label %bb6.i.i.i11.i

bb6.i.i.i11.i:                                    ; preds = %.noexc.i.i.i, %<core::array::iter::iter_inner::PolymorphicIter<[core::mem::maybe_uninit::MaybeUninit<&str>]>>::try_fold::<(), core::iter::traits::iterator::Iterator::try_for_each::call<&str, core::result::Result<(), pyo3::err::PyErr>, purrdf_native::py_store::presentation::settled<pyo3::instance::Py<pyo3::types::any::PyAny>, <purrdf_native::py_store::quad_store::PyQuadStore>::query::{closure#0}>::{closure#0}::{closure#0}::{closure#0}>::{closure#0}, core::result::Result<(), pyo3::err::PyErr>>::{closure#0} (.exit.thread11.i.i.i.i)
  call void @llvm.lifetime.end.p0(ptr nonnull %_9.i.i.i.i), !noalias !107984
  call void @llvm.lifetime.start.p0(ptr nonnull %_9.i.i.i.i), !noalias !107984
  call void @llvm.lifetime.start.p0(ptr nonnull %_4.i.i.i.i.i.i.i), !noalias !107989
; invoke <pyo3::types::string::PyString>::new
  %_3.i.i.i.i.i.i4.1.i.i.i = invoke noundef nonnull ptr @<pyo3::types::string::PyString>::new(ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @alloc_6f6b49b405cc516c15819f32fccb7974, i64 noundef 12)
          to label %_3.i.i.i.i.i.i.noexc.1.i.i.i unwind label %cleanup1.i.i10.i, !noalias !107981

_3.i.i.i.i.i.i.noexc.1.i.i.i:                     ; preds = %bb6.i.i.i11.i
; invoke <pyo3::instance::Bound<pyo3::types::any::PyAny> as pyo3::types::any::PyAnyMethods>::hasattr::inner
  invoke void @<pyo3::instance::Bound<pyo3::types::any::PyAny> as pyo3::types::any::PyAnyMethods>::hasattr::inner(ptr noalias nofree noundef nonnull sret([56 x i8]) align 8 captures(none) dereferenceable(56) %_4.i.i.i.i.i.i.i, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(8) %value.i.i.i, ptr noundef nonnull %_3.i.i.i.i.i.i4.1.i.i.i)
          to label %<pyo3::instance::Bound<pyo3::types::any::PyAny> as pyo3::types::any::PyAnyMethods>::hasattr::<&str> (.exit.i.i.i.i.1.i.i.i) unwind label %bb8.i.i.i.i.i.i.i.i, !noalias !107996

<pyo3::instance::Bound<pyo3::types::any::PyAny> as pyo3::types::any::PyAnyMethods>::hasattr::<&str> (.exit.i.i.i.i.1.i.i.i): ; preds = %_3.i.i.i.i.i.i.noexc.1.i.i.i
  call void @_Py_DecRef(ptr noundef nonnull %_3.i.i.i.i.i.i4.1.i.i.i) #77, !noalias !107999
  %220 = load i8, ptr %_4.i.i.i.i.i.i.i, align 8, !range !4704, !noalias !108002, !noundef !3892
  %221 = trunc nuw i8 %220 to i1
  br i1 %221, label %<core::array::iter::iter_inner::PolymorphicIter<[core::mem::maybe_uninit::MaybeUninit<&str>]>>::try_fold::<(), core::iter::traits::iterator::Iterator::try_for_each::call<&str, core::result::Result<(), pyo3::err::PyErr>, purrdf_native::py_store::presentation::settled<pyo3::instance::Py<pyo3::types::any::PyAny>, <purrdf_native::py_store::quad_store::PyQuadStore>::query::{closure#0}>::{closure#0}::{closure#0}::{closure#0}>::{closure#0}, core::result::Result<(), pyo3::err::PyErr>>::{closure#0} (.exit.thread.i.i.i.i), label %bb8.i.i.i.i.1.i.i.i

bb8.i.i.i.i.1.i.i.i:                              ; preds = %<pyo3::instance::Bound<pyo3::types::any::PyAny> as pyo3::types::any::PyAnyMethods>::hasattr::<&str> (.exit.i.i.i.i.1.i.i.i)
  %222 = load i8, ptr %212, align 1, !range !4704, !noalias !108002, !noundef !3892
  %_16.i.i.i.i.1.i.i.i = trunc nuw i8 %222 to i1
  br i1 %_16.i.i.i.i.1.i.i.i, label %<core::array::iter::iter_inner::PolymorphicIter<[core::mem::maybe_uninit::MaybeUninit<&str>]>>::try_fold::<(), core::iter::traits::iterator::Iterator::try_for_each::call<&str, core::result::Result<(), pyo3::err::PyErr>, purrdf_native::py_store::presentation::settled<pyo3::instance::Py<pyo3::types::any::PyAny>, <purrdf_native::py_store::quad_store::PyQuadStore>::query::{closure#0}>::{closure#0}::{closure#0}::{closure#0}>::{closure#0}, core::result::Result<(), pyo3::err::PyErr>>::{closure#0} (.exit.thread11.i.1.i.i.i), label %<core::array::iter::iter_inner::PolymorphicIter<[core::mem::maybe_uninit::MaybeUninit<&str>]>>::try_fold::<(), core::iter::traits::iterator::Iterator::try_for_each::call<&str, core::result::Result<(), pyo3::err::PyErr>, purrdf_native::py_store::presentation::settled<pyo3::instance::Py<pyo3::types::any::PyAny>, <purrdf_native::py_store::quad_store::PyQuadStore>::query::{closure#0}>::{closure#0}::{closure#0}::{closure#0}>::{closure#0}, core::result::Result<(), pyo3::err::PyErr>>::{closure#0} (.exit.i.1.i.i.i)

<core::array::iter::iter_inner::PolymorphicIter<[core::mem::maybe_uninit::MaybeUninit<&str>]>>::try_fold::<(), core::iter::traits::iterator::Iterator::try_for_each::call<&str, core::result::Result<(), pyo3::err::PyErr>, purrdf_native::py_store::presentation::settled<pyo3::instance::Py<pyo3::types::any::PyAny>, <purrdf_native::py_store::quad_store::PyQuadStore>::query::{closure#0}>::{closure#0}::{closure#0}::{closure#0}>::{closure#0}, core::result::Result<(), pyo3::err::PyErr>>::{closure#0} (.exit.i.1.i.i.i): ; preds = %bb8.i.i.i.i.1.i.i.i
  %_26.i.i.i.i.1.i.i.i = call noundef ptr @Py_GetConstantBorrowed(i32 noundef 0) #77, !noalias !107996
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %_26.i.i.i.i.1.i.i.i) ]
  call void @_Py_IncRef(ptr noundef nonnull %_26.i.i.i.i.1.i.i.i) #77, !noalias !107996
; invoke <pyo3::instance::Bound<pyo3::types::any::PyAny> as pyo3::types::any::PyAnyMethods>::setattr::<&str, pyo3::instance::Py<pyo3::types::any::PyAny>>
  invoke fastcc void @<pyo3::instance::Bound<pyo3::types::any::PyAny> as pyo3::types::any::PyAnyMethods>::setattr::<&str, pyo3::instance::Py<pyo3::types::any::PyAny>>(ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(56) %_9.i.i.i.i, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(8) %value.i.i.i, ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @alloc_6f6b49b405cc516c15819f32fccb7974, i64 noundef 12, ptr noundef nonnull %_26.i.i.i.i.1.i.i.i)
          to label %.noexc.1.i.i.i unwind label %cleanup1.i.i10.i, !noalias !107981

.noexc.1.i.i.i:                                   ; preds = %<core::array::iter::iter_inner::PolymorphicIter<[core::mem::maybe_uninit::MaybeUninit<&str>]>>::try_fold::<(), core::iter::traits::iterator::Iterator::try_for_each::call<&str, core::result::Result<(), pyo3::err::PyErr>, purrdf_native::py_store::presentation::settled<pyo3::instance::Py<pyo3::types::any::PyAny>, <purrdf_native::py_store::quad_store::PyQuadStore>::query::{closure#0}>::{closure#0}::{closure#0}::{closure#0}>::{closure#0}, core::result::Result<(), pyo3::err::PyErr>>::{closure#0} (.exit.i.1.i.i.i)
  %_2.i.pre.i.1.i.i.i = load i64, ptr %_9.i.i.i.i, align 8, !range !6076, !alias.scope !108005, !noalias !108008
  %223 = trunc nuw i64 %_2.i.pre.i.1.i.i.i to i1
  call void @llvm.lifetime.end.p0(ptr nonnull %_4.i.i.i.i.i.i.i), !noalias !107989
  br i1 %223, label %bb5.i.i.i12.i, label %bb4.i6.i.i.i

<core::array::iter::iter_inner::PolymorphicIter<[core::mem::maybe_uninit::MaybeUninit<&str>]>>::try_fold::<(), core::iter::traits::iterator::Iterator::try_for_each::call<&str, core::result::Result<(), pyo3::err::PyErr>, purrdf_native::py_store::presentation::settled<pyo3::instance::Py<pyo3::types::any::PyAny>, <purrdf_native::py_store::quad_store::PyQuadStore>::query::{closure#0}>::{closure#0}::{closure#0}::{closure#0}>::{closure#0}, core::result::Result<(), pyo3::err::PyErr>>::{closure#0} (.exit.thread11.i.1.i.i.i): ; preds = %bb8.i.i.i.i.1.i.i.i
  call void @llvm.lifetime.end.p0(ptr nonnull %_4.i.i.i.i.i.i.i), !noalias !107989
  br label %bb4.i6.i.i.i

cleanup1.i.i10.i:                                 ; preds = %<core::array::iter::iter_inner::PolymorphicIter<[core::mem::maybe_uninit::MaybeUninit<&str>]>>::try_fold::<(), core::iter::traits::iterator::Iterator::try_for_each::call<&str, core::result::Result<(), pyo3::err::PyErr>, purrdf_native::py_store::presentation::settled<pyo3::instance::Py<pyo3::types::any::PyAny>, <purrdf_native::py_store::quad_store::PyQuadStore>::query::{closure#0}>::{closure#0}::{closure#0}::{closure#0}>::{closure#0}, core::result::Result<(), pyo3::err::PyErr>>::{closure#0} (.exit.i.1.i.i.i), %bb6.i.i.i11.i, %<core::array::iter::iter_inner::PolymorphicIter<[core::mem::maybe_uninit::MaybeUninit<&str>]>>::try_fold::<(), core::iter::traits::iterator::Iterator::try_for_each::call<&str, core::result::Result<(), pyo3::err::PyErr>, purrdf_native::py_store::presentation::settled<pyo3::instance::Py<pyo3::types::any::PyAny>, <purrdf_native::py_store::quad_store::PyQuadStore>::query::{closure#0}>::{closure#0}::{closure#0}::{closure#0}>::{closure#0}, core::result::Result<(), pyo3::err::PyErr>>::{closure#0} (.exit.i.i.i.i), %bb2.lr.ph.i.i.i.i
  %224 = landingpad { ptr, i32 }
          cleanup
  br label %bb12.i.i.i

bb4.i6.i.i.i:                                     ; preds = %<core::array::iter::iter_inner::PolymorphicIter<[core::mem::maybe_uninit::MaybeUninit<&str>]>>::try_fold::<(), core::iter::traits::iterator::Iterator::try_for_each::call<&str, core::result::Result<(), pyo3::err::PyErr>, purrdf_native::py_store::presentation::settled<pyo3::instance::Py<pyo3::types::any::PyAny>, <purrdf_native::py_store::quad_store::PyQuadStore>::query::{closure#0}>::{closure#0}::{closure#0}::{closure#0}>::{closure#0}, core::result::Result<(), pyo3::err::PyErr>>::{closure#0} (.exit.thread11.i.1.i.i.i), %.noexc.1.i.i.i
  call void @llvm.lifetime.end.p0(ptr nonnull %_9.i.i.i.i), !noalias !107984
  %_13.sroa.4.0._1.sroa_idx.i7.i.i = getelementptr inbounds nuw i8, ptr %_3.i1.i, i64 16
  %_13.sroa.4.0.copyload.i8.i.i = load i64, ptr %_13.sroa.4.0._1.sroa_idx.i7.i.i, align 8, !alias.scope !107978, !noalias !107981
  %_13.sroa.5.0._1.sroa_idx.i9.i.i = getelementptr inbounds nuw i8, ptr %_3.i1.i, i64 24
  %225 = load <2 x ptr>, ptr %_13.sroa.5.0._1.sroa_idx.i9.i.i, align 8, !alias.scope !107978, !noalias !107981
  %_13.sroa.7.0._1.sroa_idx.i.i.i = getelementptr inbounds nuw i8, ptr %_3.i1.i, i64 40
  %_13.sroa.7.0.copyload.i.i.i = load i64, ptr %_13.sroa.7.0._1.sroa_idx.i.i.i, align 8, !alias.scope !107978, !noalias !107981
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %_4.sroa.0.i, ptr noundef nonnull align 8 dereferenceable(16) %_3.i1.i, i64 16, i1 false), !alias.scope !107982, !noalias !107983
  br label %<pyo3::marker::Python>::attach::<purrdf_native::py_store::presentation::settled<pyo3::instance::Py<pyo3::types::any::PyAny>, <purrdf_native::py_store::quad_store::PyQuadStore>::query::{closure#0}>::{closure#0}::{closure#0}, pyo3::err::PyErr> (.exit.i)

bb5.i.i.i12.i:                                    ; preds = %.noexc.1.i.i.i, %.noexc.i.i.i, %<core::array::iter::iter_inner::PolymorphicIter<[core::mem::maybe_uninit::MaybeUninit<&str>]>>::try_fold::<(), core::iter::traits::iterator::Iterator::try_for_each::call<&str, core::result::Result<(), pyo3::err::PyErr>, purrdf_native::py_store::presentation::settled<pyo3::instance::Py<pyo3::types::any::PyAny>, <purrdf_native::py_store::quad_store::PyQuadStore>::query::{closure#0}>::{closure#0}::{closure#0}::{closure#0}>::{closure#0}, core::result::Result<(), pyo3::err::PyErr>>::{closure#0} (.exit.thread.i.i.i.i)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %_4.sroa.0.i, ptr noundef nonnull align 8 dereferenceable(16) %214, i64 16, i1 false), !noalias !108010
  %_4.sroa.6.0..sroa_idx16.i = getelementptr inbounds nuw i8, ptr %_9.i.i.i.i, i64 24
  %_4.sroa.6.0.copyload17.i = load i64, ptr %_4.sroa.6.0..sroa_idx16.i, align 8, !noalias !108010
  %_4.sroa.7.0..sroa_idx19.i = getelementptr inbounds nuw i8, ptr %_9.i.i.i.i, i64 32
  %226 = load <2 x ptr>, ptr %_4.sroa.7.0..sroa_idx19.i, align 8, !noalias !108010
  %_4.sroa.9.0..sroa_idx25.i = getelementptr inbounds nuw i8, ptr %_9.i.i.i.i, i64 48
  %_4.sroa.9.0.copyload26.i = load i64, ptr %_4.sroa.9.0..sroa_idx25.i, align 8, !noalias !108010
  call void @llvm.lifetime.end.p0(ptr nonnull %_9.i.i.i.i), !noalias !107984
  %_13.sroa.4.0._1.sroa_idx.i.i.i = getelementptr inbounds nuw i8, ptr %_3.i1.i, i64 16
  %_13.sroa.4.0.copyload.i.i.i = load i64, ptr %_13.sroa.4.0._1.sroa_idx.i.i.i, align 8, !alias.scope !107978, !noalias !107981
  %_13.sroa.5.0._1.sroa_idx.i.i.i = getelementptr inbounds nuw i8, ptr %_3.i1.i, i64 24
  %_13.sroa.5.0.copyload.i.i.i = load ptr, ptr %_13.sroa.5.0._1.sroa_idx.i.i.i, align 8, !alias.scope !107978, !noalias !107981
  %_13.sroa.6.0._1.sroa_idx.i.i.i = getelementptr inbounds nuw i8, ptr %_3.i1.i, i64 32
  %_13.sroa.6.0.copyload.i.i.i = load ptr, ptr %_13.sroa.6.0._1.sroa_idx.i.i.i, align 8, !alias.scope !107978, !noalias !107981
  %227 = icmp eq i64 %_13.sroa.4.0.copyload.i.i.i, 0
  br i1 %227, label %<pyo3::marker::Python>::attach::<purrdf_native::py_store::presentation::settled<pyo3::instance::Py<pyo3::types::any::PyAny>, <purrdf_native::py_store::quad_store::PyQuadStore>::query::{closure#0}>::{closure#0}::{closure#0}, pyo3::err::PyErr> (.exit.i), label %bb2.i.i.i.i.i.i13.i

bb2.i.i.i.i.i.i13.i:                              ; preds = %bb5.i.i.i12.i
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %_13.sroa.6.0.copyload.i.i.i) ]
  %.not.i.i.i.i.i.i.i14.i = icmp eq ptr %_13.sroa.5.0.copyload.i.i.i, null
  br i1 %.not.i.i.i.i.i.i.i14.i, label %bb3.i.i.i.i.i.i.i.i, label %bb2.i.i.i.i.i.i.i15.i

bb2.i.i.i.i.i.i.i15.i:                            ; preds = %bb2.i.i.i.i.i.i13.i
  %228 = load ptr, ptr %_13.sroa.6.0.copyload.i.i.i, align 8, !invariant.load !3892, !noalias !108011
  %.not.i.i.i.i.i.i.i.i.i = icmp eq ptr %228, null
  br i1 %.not.i.i.i.i.i.i.i.i.i, label %bb3.i.i.i.i.i.i.i.i.i, label %is_not_null.i.i.i.i.i.i.i.i.i

is_not_null.i.i.i.i.i.i.i.i.i:                    ; preds = %bb2.i.i.i.i.i.i.i15.i
  invoke void %228(ptr noundef nonnull %_13.sroa.5.0.copyload.i.i.i)
          to label %bb3.i.i.i.i.i.i.i.i.i unwind label %cleanup.i.i.i.i.i.i.i.i.i, !noalias !108011

bb3.i.i.i.i.i.i.i.i.i:                            ; preds = %is_not_null.i.i.i.i.i.i.i.i.i, %bb2.i.i.i.i.i.i.i15.i
  %229 = getelementptr inbounds nuw i8, ptr %_13.sroa.6.0.copyload.i.i.i, i64 8
  %size.i.i.i.i.i.i.i.i.i.i = load i64, ptr %229, align 8, !range !4107, !invariant.load !3892, !noalias !108011
  %230 = icmp eq i64 %size.i.i.i.i.i.i.i.i.i.i, 0
  br i1 %230, label %<pyo3::marker::Python>::attach::<purrdf_native::py_store::presentation::settled<pyo3::instance::Py<pyo3::types::any::PyAny>, <purrdf_native::py_store::quad_store::PyQuadStore>::query::{closure#0}>::{closure#0}::{closure#0}, pyo3::err::PyErr> (.exit.i), label %<alloc::alloc::Global as core::alloc::Allocator>::deallocate (.exit.i.i.i.i.i.i.i.i.i.i)

<alloc::alloc::Global as core::alloc::Allocator>::deallocate (.exit.i.i.i.i.i.i.i.i.i.i): ; preds = %bb3.i.i.i.i.i.i.i.i.i
  %231 = getelementptr inbounds nuw i8, ptr %_13.sroa.6.0.copyload.i.i.i, i64 16
  %align.i.i.i.i.i.i.i.i.i.i = load i64, ptr %231, align 8, !range !3894, !invariant.load !3892, !noalias !108011
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %_13.sroa.5.0.copyload.i.i.i, i64 noundef %size.i.i.i.i.i.i.i.i.i.i, i64 noundef range(i64 1, -9223372036854775807) %align.i.i.i.i.i.i.i.i.i.i) #77, !noalias !108011
  br label %<pyo3::marker::Python>::attach::<purrdf_native::py_store::presentation::settled<pyo3::instance::Py<pyo3::types::any::PyAny>, <purrdf_native::py_store::quad_store::PyQuadStore>::query::{closure#0}>::{closure#0}::{closure#0}, pyo3::err::PyErr> (.exit.i)

cleanup.i.i.i.i.i.i.i.i.i:                        ; preds = %is_not_null.i.i.i.i.i.i.i.i.i
  %232 = landingpad { ptr, i32 }
          cleanup
  %233 = getelementptr inbounds nuw i8, ptr %_13.sroa.6.0.copyload.i.i.i, i64 8
  %size.i4.i.i.i.i.i.i.i.i.i = load i64, ptr %233, align 8, !range !4107, !invariant.load !3892, !noalias !108011
  %234 = icmp eq i64 %size.i4.i.i.i.i.i.i.i.i.i, 0
  br i1 %234, label %cleanup1.body.i.i, label %<alloc::alloc::Global as core::alloc::Allocator>::deallocate (.exit.i5.i.i.i.i.i.i.i.i.i)

<alloc::alloc::Global as core::alloc::Allocator>::deallocate (.exit.i5.i.i.i.i.i.i.i.i.i): ; preds = %cleanup.i.i.i.i.i.i.i.i.i
  %235 = getelementptr inbounds nuw i8, ptr %_13.sroa.6.0.copyload.i.i.i, i64 16
  %align.i6.i.i.i.i.i.i.i.i.i = load i64, ptr %235, align 8, !range !3894, !invariant.load !3892, !noalias !108011
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %_13.sroa.5.0.copyload.i.i.i, i64 noundef %size.i4.i.i.i.i.i.i.i.i.i, i64 noundef range(i64 1, -9223372036854775807) %align.i6.i.i.i.i.i.i.i.i.i) #77, !noalias !108011
  br label %cleanup1.body.i.i

bb3.i.i.i.i.i.i.i.i:                              ; preds = %bb2.i.i.i.i.i.i13.i
  %_3.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @pyo3::internal::state::ATTACH_COUNT::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL)
  %self3.val.i.i.i.i.i.i.i.i.i.i.i.i.i = load i64, ptr %_3.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, align 8, !noalias !108020, !noundef !3892
  %_0.i.i.i.i.i.i.i.i.i.i.i.i.i.i = icmp sgt i64 %self3.val.i.i.i.i.i.i.i.i.i.i.i.i.i, 0
  br i1 %_0.i.i.i.i.i.i.i.i.i.i.i.i.i.i, label %bb1.i.i.i.i.i.i.i.i.i.i.i.i, label %bb2.i.i.i.i.i.i.i.i.i.i.i.i, !prof !7253

bb2.i.i.i.i.i.i.i.i.i.i.i.i:                      ; preds = %bb3.i.i.i.i.i.i.i.i
; invoke <pyo3::instance::Py<_> as core::ops::drop::Drop>::drop::drop_slow
  invoke void @<pyo3::instance::Py<_> as core::ops::drop::Drop>::drop::drop_slow(ptr noundef nonnull %_13.sroa.6.0.copyload.i.i.i)
          to label %<pyo3::marker::Python>::attach::<purrdf_native::py_store::presentation::settled<pyo3::instance::Py<pyo3::types::any::PyAny>, <purrdf_native::py_store::quad_store::PyQuadStore>::query::{closure#0}>::{closure#0}::{closure#0}, pyo3::err::PyErr> (.exit.i) unwind label %cleanup1.i.i, !noalias !107971

bb1.i.i.i.i.i.i.i.i.i.i.i.i:                      ; preds = %bb3.i.i.i.i.i.i.i.i
  call void @_Py_DecRef(ptr noundef nonnull %_13.sroa.6.0.copyload.i.i.i) #77, !noalias !108011
  br label %<pyo3::marker::Python>::attach::<purrdf_native::py_store::presentation::settled<pyo3::instance::Py<pyo3::types::any::PyAny>, <purrdf_native::py_store::quad_store::PyQuadStore>::query::{closure#0}>::{closure#0}::{closure#0}, pyo3::err::PyErr> (.exit.i)

terminate.i.i8.i:                                 ; preds = %bb12.i.i.i
  %236 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #75, !noalias !107981
  unreachable

bb12.i.i.i:                                       ; preds = %cleanup1.i.i10.i, %bb8.i.i.i.i.i.i.i.i, %cleanup.i.i.i, %bb4.i.i.i7.i
  %.pn.ph.i.i.i = phi { ptr, i32 } [ %209, %bb4.i.i.i7.i ], [ %210, %cleanup.i.i.i ], [ %224, %cleanup1.i.i10.i ], [ %215, %bb8.i.i.i.i.i.i.i.i ]
; invoke core::ptr::drop_glue::<pyo3::err::PyErr>
  invoke void @core::ptr::drop_glue::<pyo3::err::PyErr>(ptr noalias nofree noundef nonnull align 8 dereferenceable(48) %_3.i1.i) #79
          to label %cleanup1.body.i.i unwind label %terminate.i.i8.i, !noalias !107981

cleanup1.i.i:                                     ; preds = %bb2.i.i.i.i.i.i.i.i.i.i.i.i
  %237 = landingpad { ptr, i32 }
          cleanup
  br label %cleanup1.body.i.i

cleanup1.body.i.i:                                ; preds = %cleanup1.i.i, %bb12.i.i.i, %<alloc::alloc::Global as core::alloc::Allocator>::deallocate (.exit.i5.i.i.i.i.i.i.i.i.i), %cleanup.i.i.i.i.i.i.i.i.i
  %eh.lpad-body.i.i = phi { ptr, i32 } [ %237, %cleanup1.i.i ], [ %.pn.ph.i.i.i, %bb12.i.i.i ], [ %232, %cleanup.i.i.i.i.i.i.i.i.i ], [ %232, %<alloc::alloc::Global as core::alloc::Allocator>::deallocate (.exit.i5.i.i.i.i.i.i.i.i.i) ]
; invoke <pyo3::internal::state::AttachGuard as core::ops::drop::Drop>::drop
  invoke void @<pyo3::internal::state::AttachGuard as core::ops::drop::Drop>::drop(ptr noalias nofree noundef nonnull align 4 dereferenceable(4) %guard.i.i)
          to label %common.resume.i unwind label %terminate.i3.i, !noalias !107971

terminate.i3.i:                                   ; preds = %bb6.i2.i, %cleanup1.body.i.i
  %238 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #75, !noalias !107971
  unreachable

bb6.i2.i:                                         ; preds = %bb4.i
  %239 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<pyo3::err::PyErr>
  invoke void @core::ptr::drop_glue::<pyo3::err::PyErr>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(48) %206)
          to label %common.resume.i unwind label %terminate.i3.i, !noalias !107644

<pyo3::marker::Python>::attach::<purrdf_native::py_store::presentation::settled<pyo3::instance::Py<pyo3::types::any::PyAny>, <purrdf_native::py_store::quad_store::PyQuadStore>::query::{closure#0}>::{closure#0}::{closure#0}, pyo3::err::PyErr> (.exit.i): ; preds = %bb1.i.i.i.i.i.i.i.i.i.i.i.i, %bb2.i.i.i.i.i.i.i.i.i.i.i.i, %<alloc::alloc::Global as core::alloc::Allocator>::deallocate (.exit.i.i.i.i.i.i.i.i.i.i), %bb3.i.i.i.i.i.i.i.i.i, %bb5.i.i.i12.i, %bb4.i6.i.i.i, %bb3.i.i.i
  %_4.sroa.9.0.i = phi i64 [ %_4.sroa.9.0.copyload27.i, %bb3.i.i.i ], [ %_4.sroa.9.0.copyload26.i, %bb5.i.i.i12.i ], [ %_4.sroa.9.0.copyload26.i, %bb1.i.i.i.i.i.i.i.i.i.i.i.i ], [ %_4.sroa.9.0.copyload26.i, %bb2.i.i.i.i.i.i.i.i.i.i.i.i ], [ %_4.sroa.9.0.copyload26.i, %bb3.i.i.i.i.i.i.i.i.i ], [ %_4.sroa.9.0.copyload26.i, %<alloc::alloc::Global as core::alloc::Allocator>::deallocate (.exit.i.i.i.i.i.i.i.i.i.i) ], [ %_13.sroa.7.0.copyload.i.i.i, %bb4.i6.i.i.i ]
  %_4.sroa.6.0.i = phi i64 [ %_4.sroa.6.0.copyload18.i, %bb3.i.i.i ], [ %_4.sroa.6.0.copyload17.i, %bb5.i.i.i12.i ], [ %_4.sroa.6.0.copyload17.i, %bb1.i.i.i.i.i.i.i.i.i.i.i.i ], [ %_4.sroa.6.0.copyload17.i, %bb2.i.i.i.i.i.i.i.i.i.i.i.i ], [ %_4.sroa.6.0.copyload17.i, %bb3.i.i.i.i.i.i.i.i.i ], [ %_4.sroa.6.0.copyload17.i, %<alloc::alloc::Global as core::alloc::Allocator>::deallocate (.exit.i.i.i.i.i.i.i.i.i.i) ], [ %_13.sroa.4.0.copyload.i8.i.i, %bb4.i6.i.i.i ]
  %240 = phi <2 x ptr> [ %211, %bb3.i.i.i ], [ %226, %bb5.i.i.i12.i ], [ %226, %bb1.i.i.i.i.i.i.i.i.i.i.i.i ], [ %226, %bb2.i.i.i.i.i.i.i.i.i.i.i.i ], [ %226, %bb3.i.i.i.i.i.i.i.i.i ], [ %226, %<alloc::alloc::Global as core::alloc::Allocator>::deallocate (.exit.i.i.i.i.i.i.i.i.i.i) ], [ %225, %bb4.i6.i.i.i ]
  call void @llvm.lifetime.end.p0(ptr nonnull %_3.i1.i), !noalias !107971
; call <pyo3::internal::state::AttachGuard as core::ops::drop::Drop>::drop
  call void @<pyo3::internal::state::AttachGuard as core::ops::drop::Drop>::drop(ptr noalias nofree noundef nonnull align 4 dereferenceable(4) %guard.i.i), !noalias !107971
  call void @llvm.lifetime.end.p0(ptr nonnull %guard.i.i), !noalias !107971
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %206, ptr noundef nonnull align 8 dereferenceable(16) %_4.sroa.0.i, i64 16, i1 false), !noalias !107644
  %_4.sroa.6.0..sroa_idx.i = getelementptr inbounds nuw i8, ptr %_0, i64 24
  store i64 %_4.sroa.6.0.i, ptr %_4.sroa.6.0..sroa_idx.i, align 8, !alias.scope !107641, !noalias !107644
  %_4.sroa.7.0..sroa_idx.i = getelementptr inbounds nuw i8, ptr %_0, i64 32
  store <2 x ptr> %240, ptr %_4.sroa.7.0..sroa_idx.i, align 8, !alias.scope !107641, !noalias !107644
  %_4.sroa.9.0..sroa_idx.i = getelementptr inbounds nuw i8, ptr %_0, i64 48
  store i64 %_4.sroa.9.0.i, ptr %_4.sroa.9.0..sroa_idx.i, align 8, !alias.scope !107641, !noalias !107644
  store i64 1, ptr %_0, align 8, !alias.scope !107641, !noalias !107644
  call void @llvm.lifetime.end.p0(ptr nonnull %_4.sroa.0.i)
  br label %purrdf_native::py_store::presentation::settled::<pyo3::instance::Py<pyo3::types::any::PyAny>, <purrdf_native::py_store::quad_store::PyQuadStore>::query::{closure#0}> (.exit)

purrdf_native::py_store::presentation::settled::<pyo3::instance::Py<pyo3::types::any::PyAny>, <purrdf_native::py_store::quad_store::PyQuadStore>::query::{closure#0}> (.exit): ; preds = %<purrdf_native::py_store::quad_store::PyQuadStore>::query::{closure#0} (.exit.i), %<pyo3::marker::Python>::attach::<purrdf_native::py_store::presentation::settled<pyo3::instance::Py<pyo3::types::any::PyAny>, <purrdf_native::py_store::quad_store::PyQuadStore>::query::{closure#0}>::{closure#0}::{closure#0}, pyo3::err::PyErr> (.exit.i)
  call void @llvm.lifetime.end.p0(ptr nonnull %_14)
  ret void
}


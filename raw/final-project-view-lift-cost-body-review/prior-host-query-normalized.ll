define internal fastcc void @<purrdf_native::py_store::quad_store::PyQuadStore>::query_governed(ptr dead_on_unwind noalias nofree noundef nonnull writable align 8 captures(none) dereferenceable(56) %_0, ptr noundef nonnull align 8 %self, ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) %query.0, i64 noundef %query.1, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable_or_null(8) %substitutions, ptr noalias nofree noundef nonnull readonly align 8 captures(none) dead_on_return dereferenceable(24) %extension_namespaces, ptr noalias nofree noundef nonnull readonly align 8 captures(none) dead_on_return dereferenceable(24) %property_fn_namespaces, ptr noalias nofree noundef nonnull readonly align 8 captures(none) dead_on_return dereferenceable(48) %standpoint_predicates, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable_or_null(8) %relations, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable_or_null(8) %relations_from_graph, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable_or_null(8) %path_relations, ptr noalias nofree noundef nonnull readonly align 8 captures(none) dead_on_return dereferenceable(24) %aggregate_namespace, ptr noalias nofree noundef readonly captures(address, read_provenance) %xpath_regex.0, i64 %xpath_regex.1, ptr noalias nofree noundef nonnull readonly align 8 captures(none) dead_on_return dereferenceable(24) %division, i64 noundef range(i64 0, 2) %fuel.0, i64 %fuel.1, i64 noundef range(i64 0, 2) %deadline_ms.0, i64 %deadline_ms.1, i64 noundef range(i64 0, 2) %max_answers.0, i64 %max_answers.1, i64 noundef range(i64 0, 2) %max_intermediate_cells.0, i64 %max_intermediate_cells.1, i64 noundef range(i64 0, 2) %max_scratch_bytes.0, i64 %max_scratch_bytes.1, i64 noundef range(i64 0, 2) %max_remote_requests.0, i64 %max_remote_requests.1, i1 noundef zeroext %no_ceiling, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable_or_null(8) %cancel) unnamed_addr #ATTR personality ptr @rust_eh_personality !guid !ID {
start:
  %_4.i.i.i.i.i.i.i = alloca [56 x i8], align 8
  %_9.i.i.i.i = alloca [56 x i8], align 8
  %_3.i1.i = alloca [48 x i8], align 8
  %guard.i.i = alloca [4 x i8], align 4
  %args.i44.i.i.i.i.i.i = alloca [16 x i8], align 8
  %_3.i45.i.i.i.i.i.i = alloca [24 x i8], align 8
  %_12.i.i.i.i.i.i.i = alloca [24 x i8], align 8
  %_2.i.i.i.i.i.i.i = alloca [24 x i8], align 8
  %args.i.i.i.i.i.i.i = alloca [16 x i8], align 8
  %_3.i.i.i.i.i.i.i = alloca [24 x i8], align 8
  %_90.i.i.i.i.i.i = alloca [96 x i8], align 8
  %_89.i.i.i.i.i.i = alloca [48 x i8], align 8
  %_47.i.i.i.i.i.i = alloca [96 x i8], align 8
  %parser_options.i.i.i.i.i.i = alloca [72 x i8], align 8
  %_31.i.i.i.i.i.i = alloca [352 x i8], align 8
  %_30.sroa.6.i.i.i.i.i.i = alloca [48 x i8], align 8
  %_29.i.i.i.i.i.i = alloca [352 x i8], align 8
  %_27.i.i.i.i.i.i = alloca [184 x i8], align 8
  %_25.i.i.i.i.i.i = alloca [48 x i8], align 8
  %_22.i.i.i.i.i.i = alloca [368 x i8], align 8
  %engine.i.i.i.i.i.i = alloca [520 x i8], align 8
  %aggregates.i.i.i.i.i.i = alloca [40 x i8], align 8
  %_11.i.i.i.i.i.i = alloca [80 x i8], align 8
  %_10.sroa.5.i.i.i.i.i.i = alloca [72 x i8], align 8
  %registry.i.i.i.i.i.i = alloca [72 x i8], align 8
  %_6.i.i.i.i.i.i = alloca [96 x i8], align 8
  %dataset.i.i.i.i.i.i = alloca [8 x i8], align 8
  %_guard.i.i.i.i = alloca [16 x i8], align 8
  %_13.i.i.i = alloca [56 x i8], align 8
  %_11.i.i.i = alloca [280 x i8], align 8
  %outcome.i.i.i = alloca [368 x i8], align 8
  %_8.i.i.i = alloca [88 x i8], align 8
  %_7.sroa.0.i.i.i = alloca [80 x i8], align 8
  %watch.i.i.i = alloca [8 x i8], align 8
  %governors.i.i.i = alloca [80 x i8], align 8
  %_26.i.i = alloca [72 x i8], align 8
  %outcome.i.i = alloca [368 x i8], align 8
  %_42.i.i = alloca [272 x i8], align 8
  %_40.sroa.8.i.i = alloca [48 x i8], align 8
  %_40.sroa.12.i.i = alloca [312 x i8], align 8
  %_39.sroa.6.i.i = alloca [48 x i8], align 8
  %args.i.i = alloca [104 x i8], align 8
  %config.sroa.0.i.i = alloca [96 x i8], align 8
  %_15.i.i = alloca [56 x i8], align 8
  %_14.sroa.5.i.i = alloca [48 x i8], align 8
  %specs.i.i = alloca [24 x i8], align 8
  %_9.i.i = alloca [56 x i8], align 8
  %_8.sroa.5.i.i = alloca [48 x i8], align 8
  %subs.i.i = alloca [24 x i8], align 8
  %_3.i.i = alloca [56 x i8], align 8
  %_4.sroa.0.i = alloca [16 x i8], align 8
  %_22 = alloca [328 x i8], align 8
  call void @llvm.lifetime.start.p0(ptr nonnull %_22)
  %0 = getelementptr inbounds nuw i8, ptr %_22, i64 96
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %0, ptr noundef nonnull align 8 dereferenceable(24) %division, i64 24, i1 false)
  %1 = getelementptr inbounds nuw i8, ptr %_22, i64 264
  store ptr %substitutions, ptr %1, align 8
  %2 = getelementptr inbounds nuw i8, ptr %_22, i64 272
  store ptr %relations, ptr %2, align 8
  %3 = getelementptr inbounds nuw i8, ptr %_22, i64 280
  store ptr %relations_from_graph, ptr %3, align 8
  %4 = getelementptr inbounds nuw i8, ptr %_22, i64 288
  store ptr %path_relations, ptr %4, align 8
  %5 = getelementptr inbounds nuw i8, ptr %_22, i64 120
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %5, ptr noundef nonnull align 8 dereferenceable(24) %extension_namespaces, i64 24, i1 false)
  %6 = getelementptr inbounds nuw i8, ptr %_22, i64 144
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %6, ptr noundef nonnull align 8 dereferenceable(24) %property_fn_namespaces, i64 24, i1 false)
  %7 = getelementptr inbounds nuw i8, ptr %_22, i64 168
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %7, ptr noundef nonnull align 8 dereferenceable(48) %standpoint_predicates, i64 48, i1 false)
  %8 = getelementptr inbounds nuw i8, ptr %_22, i64 296
  store ptr %xpath_regex.0, ptr %8, align 8
  %9 = getelementptr inbounds nuw i8, ptr %_22, i64 304
  store i64 %xpath_regex.1, ptr %9, align 8
  store i64 %fuel.0, ptr %_22, align 8
  %10 = getelementptr inbounds nuw i8, ptr %_22, i64 8
  store i64 %fuel.1, ptr %10, align 8
  %11 = getelementptr inbounds nuw i8, ptr %_22, i64 16
  store i64 %deadline_ms.0, ptr %11, align 8
  %12 = getelementptr inbounds nuw i8, ptr %_22, i64 24
  store i64 %deadline_ms.1, ptr %12, align 8
  %13 = getelementptr inbounds nuw i8, ptr %_22, i64 32
  store i64 %max_answers.0, ptr %13, align 8
  %14 = getelementptr inbounds nuw i8, ptr %_22, i64 40
  store i64 %max_answers.1, ptr %14, align 8
  %15 = getelementptr inbounds nuw i8, ptr %_22, i64 48
  store i64 %max_intermediate_cells.0, ptr %15, align 8
  %16 = getelementptr inbounds nuw i8, ptr %_22, i64 56
  store i64 %max_intermediate_cells.1, ptr %16, align 8
  %17 = getelementptr inbounds nuw i8, ptr %_22, i64 64
  store i64 %max_scratch_bytes.0, ptr %17, align 8
  %18 = getelementptr inbounds nuw i8, ptr %_22, i64 72
  store i64 %max_scratch_bytes.1, ptr %18, align 8
  %19 = getelementptr inbounds nuw i8, ptr %_22, i64 80
  store i64 %max_remote_requests.0, ptr %19, align 8
  %20 = getelementptr inbounds nuw i8, ptr %_22, i64 88
  store i64 %max_remote_requests.1, ptr %20, align 8
  %21 = getelementptr inbounds nuw i8, ptr %_22, i64 320
  %22 = zext i1 %no_ceiling to i8
  store i8 %22, ptr %21, align 8
  %23 = getelementptr inbounds nuw i8, ptr %_22, i64 240
  store ptr %self, ptr %23, align 8
  %24 = getelementptr inbounds nuw i8, ptr %_22, i64 312
  store ptr %cancel, ptr %24, align 8
  %25 = getelementptr inbounds nuw i8, ptr %_22, i64 216
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %25, ptr noundef nonnull align 8 dereferenceable(24) %aggregate_namespace, i64 24, i1 false)
  %26 = getelementptr inbounds nuw i8, ptr %_22, i64 248
  store ptr %query.0, ptr %26, align 8
  %27 = getelementptr inbounds nuw i8, ptr %_22, i64 256
  store i64 %query.1, ptr %27, align 8
  tail call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  call void @llvm.lifetime.start.p0(ptr nonnull %_26.i.i), !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %outcome.i.i), !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %_3.i.i), !noalias !ID
  %28 = load i64, ptr %0, align 8, !range !ID, !alias.scope !ID, !noalias !ID, !noundef !ID
  %.not.i.i = icmp eq i64 %28, -1
  %29 = getelementptr inbounds nuw i8, ptr %_22, i64 104
  %_59.i.i = load ptr, ptr %29, align 8, !alias.scope !ID, !noalias !ID, !nonnull !ID
  %30 = getelementptr inbounds nuw i8, ptr %_22, i64 112
  %_58.i.i = load i64, ptr %30, align 8, !alias.scope !ID, !noalias !ID
  %_4.sroa.5.0.i.i = select i1 %.not.i.i, i64 undef, i64 %_58.i.i
  %_4.sroa.0.0.i.i = select i1 %.not.i.i, ptr null, ptr %_59.i.i
; invoke purrdf_native::py_store::env::division_policy
  invoke fastcc void @purrdf_native::py_store::env::division_policy(ptr noalias nofree noundef align 8 captures(address) dereferenceable(56) %_3.i.i, ptr noalias nofree noundef readonly captures(address, read_provenance) %_4.sroa.0.0.i.i, i64 %_4.sroa.5.0.i.i)
          to label %bb1.i.i unwind label %cleanup.i.i, !noalias !ID

bb47.i.i:                                         ; preds = %bb37.i.i, %cleanup9.i.i, %bb17.i.i.i, %bb2.i.i.i.i26.i.i.i, %bb2.i.i24.i.i.i, %bb15.i.i.i, %cleanup.i.i
  %_48.sroa.0.0.i.i = phi i1 [ true, %cleanup.i.i ], [ true, %bb37.i.i ], [ false, %cleanup9.i.i ], [ false, %bb17.i.i.i ], [ false, %bb2.i.i.i.i26.i.i.i ], [ false, %bb2.i.i24.i.i.i ], [ false, %bb15.i.i.i ]
  %_51.sroa.0.0.i.i = phi i8 [ %_51.sroa.0.1.i.i, %cleanup.i.i ], [ %_51.sroa.0.2.ph.i.i, %bb37.i.i ], [ 0, %cleanup9.i.i ], [ 0, %bb17.i.i.i ], [ 0, %bb2.i.i.i.i26.i.i.i ], [ 0, %bb2.i.i24.i.i.i ], [ 0, %bb15.i.i.i ]
  %.pn35.i.i = phi { ptr, i32 } [ %32, %cleanup.i.i ], [ %.pn33.ph.i.i, %bb37.i.i ], [ %225, %cleanup9.i.i ], [ %lpad.thr_comm.split-lp.i.i.i, %bb17.i.i.i ], [ %.pn6.i.i.i, %bb2.i.i.i.i26.i.i.i ], [ %.pn6.i.i.i, %bb2.i.i24.i.i.i ], [ %.pn6.i.i.i, %bb15.i.i.i ]
  %31 = icmp sgt i64 %28, 0
  br i1 %31, label %bb2.i.i.i4.i.i.i.i.i, label %bb45.i.i

bb2.i.i.i4.i.i.i.i.i:                             ; preds = %bb47.i.i
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %_59.i.i, i64 noundef %28, i64 noundef range(i64 1, -9223372036854775807) 1) #ATTR, !noalias !ID
  br label %bb45.i.i

cleanup.i.i:                                      ; preds = %bb62.i.i, %bb52.i.i, %start
  %_51.sroa.0.1.i.i = phi i8 [ %_51.sroa.0.5.i.i, %bb62.i.i ], [ 1, %bb52.i.i ], [ 1, %start ]
  %32 = landingpad { ptr, i32 }
          cleanup
  br label %bb47.i.i

bb1.i.i:                                          ; preds = %start
  %33 = load i32, ptr %_3.i.i, align 8, !range !ID, !noalias !ID, !noundef !ID
  %34 = trunc nuw i32 %33 to i1
  br i1 %34, label %bb51.i.i, label %bb52.i.i

bb51.i.i:                                         ; preds = %bb1.i.i
  %35 = getelementptr inbounds nuw i8, ptr %_3.i.i, i64 8
  %_62.sroa.0.0.copyload.i.i = load i32, ptr %35, align 8, !noalias !ID
  %_62.sroa.4.0..sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %_3.i.i, i64 12
  %_65.sroa.4.0..sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %_0, i64 12
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 4 dereferenceable(44) %_65.sroa.4.0..sroa_idx.i.i, ptr noundef nonnull align 4 dereferenceable(44) %_62.sroa.4.0..sroa_idx.i.i, i64 44, i1 false), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_3.i.i), !noalias !ID
  %36 = getelementptr inbounds nuw i8, ptr %_0, i64 8
  store i32 %_62.sroa.0.0.copyload.i.i, ptr %36, align 8, !alias.scope !ID, !noalias !ID
  store i64 1, ptr %_0, align 8, !alias.scope !ID, !noalias !ID
  br label %bb61.i.i

bb52.i.i:                                         ; preds = %bb1.i.i
  %37 = getelementptr inbounds nuw i8, ptr %_3.i.i, i64 4
  %_61.sroa.0.0.copyload.i.i = load i64, ptr %37, align 4, !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_3.i.i), !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %subs.i.i), !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %_8.sroa.5.i.i)
  call void @llvm.lifetime.start.p0(ptr nonnull %_9.i.i), !noalias !ID
; invoke purrdf_native::py_store::quad_store::collect_substitutions
  invoke fastcc void @purrdf_native::py_store::quad_store::collect_substitutions(ptr noalias nofree noundef align 8 captures(none) dereferenceable(56) %_9.i.i, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable_or_null(8) %substitutions)
          to label %bb3.i.i unwind label %cleanup.i.i, !noalias !ID

bb3.i.i:                                          ; preds = %bb52.i.i
  %_67.i.i = load i64, ptr %_9.i.i, align 8, !range !ID, !noalias !ID, !noundef !ID
  %38 = trunc nuw i64 %_67.i.i to i1
  %39 = getelementptr inbounds nuw i8, ptr %_9.i.i, i64 8
  br i1 %38, label %bb53.i.i, label %bb54.i.i

bb53.i.i:                                         ; preds = %bb3.i.i
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %_8.sroa.5.i.i, ptr noundef nonnull align 8 dereferenceable(48) %39, i64 48, i1 false), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_9.i.i), !noalias !ID
  %40 = getelementptr inbounds nuw i8, ptr %_0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %40, ptr noundef nonnull align 8 dereferenceable(48) %_8.sroa.5.i.i, i64 48, i1 false), !noalias !ID
  store i64 1, ptr %_0, align 8, !alias.scope !ID, !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_8.sroa.5.i.i)
  call void @llvm.lifetime.end.p0(ptr nonnull %subs.i.i), !noalias !ID
  br label %bb61.i.i

bb54.i.i:                                         ; preds = %bb3.i.i
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %_8.sroa.5.i.i, ptr noundef nonnull align 8 dereferenceable(24) %39, i64 24, i1 false), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_9.i.i), !noalias !ID
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %subs.i.i, ptr noundef nonnull align 8 dereferenceable(24) %_8.sroa.5.i.i, i64 24, i1 false), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_8.sroa.5.i.i)
  call void @llvm.lifetime.start.p0(ptr nonnull %specs.i.i), !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %_14.sroa.5.i.i)
  call void @llvm.lifetime.start.p0(ptr nonnull %_15.i.i), !noalias !ID
; invoke purrdf_native::py_store::query::collect_relations
  invoke fastcc void @purrdf_native::py_store::query::collect_relations(ptr noalias nofree noundef align 8 captures(none) dereferenceable(56) %_15.i.i, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable_or_null(8) %relations, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable_or_null(8) %relations_from_graph, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable_or_null(8) %path_relations)
          to label %bb4.i.i unwind label %cleanup7.i.i, !noalias !ID

cleanup7.i.i:                                     ; preds = %bb54.i.i
  %41 = landingpad { ptr, i32 }
          cleanup
  br label %bb37.i.i

bb4.i.i:                                          ; preds = %bb54.i.i
  %_74.i.i = load i64, ptr %_15.i.i, align 8, !range !ID, !noalias !ID, !noundef !ID
  %42 = trunc nuw i64 %_74.i.i to i1
  %43 = getelementptr inbounds nuw i8, ptr %_15.i.i, i64 8
  br i1 %42, label %bb55.i.i, label %bb56.i.i

bb55.i.i:                                         ; preds = %bb4.i.i
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %_14.sroa.5.i.i, ptr noundef nonnull align 8 dereferenceable(48) %43, i64 48, i1 false), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_15.i.i), !noalias !ID
  %44 = getelementptr inbounds nuw i8, ptr %_0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %44, ptr noundef nonnull align 8 dereferenceable(48) %_14.sroa.5.i.i, i64 48, i1 false), !noalias !ID
  store i64 1, ptr %_0, align 8, !alias.scope !ID, !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_14.sroa.5.i.i)
  br label %bb62.i.i

bb56.i.i:                                         ; preds = %bb4.i.i
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %_14.sroa.5.i.i, ptr noundef nonnull align 8 dereferenceable(24) %43, i64 24, i1 false), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_15.i.i), !noalias !ID
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %specs.i.i, ptr noundef nonnull align 8 dereferenceable(24) %_14.sroa.5.i.i, i64 24, i1 false), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_14.sroa.5.i.i)
  call void @llvm.lifetime.start.p0(ptr nonnull %config.sroa.0.i.i)
  %_22.i.sroa.0.0.copyload.i = load i64, ptr %5, align 8, !alias.scope !ID, !noalias !ID
  %_22.i.sroa.5.0..sroa_idx.i = getelementptr inbounds nuw i8, ptr %_22, i64 128
  %_22.i.sroa.5.0.copyload.i = load ptr, ptr %_22.i.sroa.5.0..sroa_idx.i, align 8, !alias.scope !ID, !noalias !ID
  %_22.i.sroa.6.0..sroa_idx.i = getelementptr inbounds nuw i8, ptr %_22, i64 136
  %_22.i.sroa.6.0.copyload.i = load i64, ptr %_22.i.sroa.6.0..sroa_idx.i, align 8, !alias.scope !ID, !noalias !ID
  %_23.i.sroa.0.0.copyload.i = load i64, ptr %6, align 8, !alias.scope !ID, !noalias !ID
  %_23.i.sroa.5.0..sroa_idx.i = getelementptr inbounds nuw i8, ptr %_22, i64 152
  %_23.i.sroa.5.0.copyload.i = load ptr, ptr %_23.i.sroa.5.0..sroa_idx.i, align 8, !alias.scope !ID, !noalias !ID
  %_23.i.sroa.6.0..sroa_idx.i = getelementptr inbounds nuw i8, ptr %_22, i64 160
  %_23.i.sroa.6.0.copyload.i = load i64, ptr %_23.i.sroa.6.0..sroa_idx.i, align 8, !alias.scope !ID, !noalias !ID
  %_24.i.sroa.0.0.copyload.i = load i64, ptr %7, align 8, !alias.scope !ID, !noalias !ID
  %_24.i.sroa.5.0..sroa_idx.i = getelementptr inbounds nuw i8, ptr %_22, i64 176
  %_24.i.sroa.5.0.copyload.i = load ptr, ptr %_24.i.sroa.5.0..sroa_idx.i, align 8, !alias.scope !ID, !noalias !ID
  %_24.i.sroa.644.0..sroa_idx.i = getelementptr inbounds nuw i8, ptr %_22, i64 192
  %_24.i.sroa.644.0.copyload.i = load i64, ptr %_24.i.sroa.644.0..sroa_idx.i, align 8, !alias.scope !ID, !noalias !ID
  %_24.i.sroa.7.0..sroa_idx.i = getelementptr inbounds nuw i8, ptr %_22, i64 200
  %_24.i.sroa.7.0.copyload.i = load ptr, ptr %_24.i.sroa.7.0..sroa_idx.i, align 8, !alias.scope !ID, !noalias !ID
; invoke purrdf_native::xpath_regex::selection
  invoke fastcc void @purrdf_native::xpath_regex::selection(ptr noalias nofree noundef align 8 captures(none) dereferenceable(72) %_26.i.i, ptr noalias nofree noundef readonly captures(address, read_provenance) %xpath_regex.0, i64 %xpath_regex.1)
          to label %bb5.i.i unwind label %cleanup8.i.i, !noalias !ID

cleanup8.i.i:                                     ; preds = %bb56.i.i
  %45 = landingpad { ptr, i32 }
          cleanup
; call core::ptr::drop_glue::<core::option::Option<purrdf_sparql_results::model::ProvenanceNamespace>>
  call fastcc void @core::ptr::drop_glue::<core::option::Option<purrdf_sparql_results::model::ProvenanceNamespace>>(ptr noalias nofree noundef readonly align 8 dereferenceable(48) %7) #ATTR, !noalias !ID
; call core::ptr::drop_glue::<core::option::Option<alloc::vec::Vec<alloc::string::String>>>
  call fastcc void @core::ptr::drop_glue::<core::option::Option<alloc::vec::Vec<alloc::string::String>>>(ptr noalias nofree noundef readonly align 8 dereferenceable(24) %6) #ATTR, !noalias !ID
; call core::ptr::drop_glue::<core::option::Option<alloc::vec::Vec<alloc::string::String>>>
  call fastcc void @core::ptr::drop_glue::<core::option::Option<alloc::vec::Vec<alloc::string::String>>>(ptr noalias nofree noundef readonly align 8 dereferenceable(24) %5) #ATTR, !noalias !ID
; invoke core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>>(ptr noalias nofree noundef align 8 dereferenceable(24) %specs.i.i) #ATTR
          to label %bb37.i.i unwind label %terminate.i.i, !noalias !ID

bb5.i.i:                                          ; preds = %bb56.i.i
  %46 = load i8, ptr %_26.i.i, align 8, !range !ID, !noalias !ID, !noundef !ID
  %47 = icmp eq i8 %46, -1
  br i1 %47, label %bb57.i.i, label %bb58.i.i

bb57.i.i:                                         ; preds = %bb5.i.i
  %48 = getelementptr inbounds nuw i8, ptr %_26.i.i, i64 8
  %49 = getelementptr inbounds nuw i8, ptr %_0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %49, ptr noundef nonnull align 8 dereferenceable(48) %48, i64 48, i1 false), !noalias !ID
  store i64 1, ptr %_0, align 8, !alias.scope !ID, !noalias !ID
  switch i64 %_24.i.sroa.0.0.copyload.i, label %bb2.i.i.i4.i.i.i.i.i.i [
    i64 -1, label %bb8.i.i
    i64 0, label %bb4.i.i.i.i
  ]

bb2.i.i.i4.i.i.i.i.i.i:                           ; preds = %bb57.i.i
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %_24.i.sroa.5.0.copyload.i) ]
; call __rustc::__rust_dealloc
  tail call void @__rustc::__rust_dealloc(ptr noundef nonnull %_24.i.sroa.5.0.copyload.i, i64 noundef %_24.i.sroa.0.0.copyload.i, i64 noundef range(i64 1, -9223372036854775807) 1) #ATTR, !noalias !ID
  br label %bb4.i.i.i.i

bb4.i.i.i.i:                                      ; preds = %bb2.i.i.i4.i.i.i.i.i.i, %bb57.i.i
  %50 = icmp eq i64 %_24.i.sroa.644.0.copyload.i, 0
  br i1 %50, label %bb8.i.i, label %bb2.i.i.i4.i.i6.i.i.i.i

bb2.i.i.i4.i.i6.i.i.i.i:                          ; preds = %bb4.i.i.i.i
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %_24.i.sroa.7.0.copyload.i) ]
; call __rustc::__rust_dealloc
  tail call void @__rustc::__rust_dealloc(ptr noundef nonnull %_24.i.sroa.7.0.copyload.i, i64 noundef %_24.i.sroa.644.0.copyload.i, i64 noundef range(i64 1, -9223372036854775807) 1) #ATTR, !noalias !ID
  br label %bb8.i.i

bb58.i.i:                                         ; preds = %bb5.i.i
  %_82.sroa.4.0._26.sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %_26.i.i, i64 1
  %_82.sroa.5.0._26.sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %_26.i.i, i64 56
  %config.sroa.10.0..sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %_42.i.i, i64 224
  call void @llvm.lifetime.start.p0(ptr nonnull %_42.i.i), !noalias !ID
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %config.sroa.10.0..sroa_idx.i.i, ptr noundef nonnull align 8 dereferenceable(16) %_82.sroa.5.0._26.sroa_idx.i.i, i64 16, i1 false), !noalias !ID
  %config.sroa.9.0..sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %_42.i.i, i64 169
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(55) %config.sroa.9.0..sroa_idx.i.i, ptr noundef nonnull align 1 dereferenceable(55) %_82.sroa.4.0._26.sroa_idx.i.i, i64 55, i1 false), !noalias !ID
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %config.sroa.0.i.i, ptr noundef nonnull readonly align 8 dereferenceable(24) %5, i64 24, i1 false), !noalias !ID
  %config.sroa.0.24..sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %config.sroa.0.i.i, i64 24
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %config.sroa.0.24..sroa_idx.i.i, ptr noundef nonnull readonly align 8 dereferenceable(24) %6, i64 24, i1 false), !noalias !ID
  %config.sroa.0.48..sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %config.sroa.0.i.i, i64 48
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %config.sroa.0.48..sroa_idx.i.i, ptr noundef nonnull readonly align 8 dereferenceable(48) %7, i64 48, i1 false), !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %args.i.i), !noalias !ID
  store i64 %fuel.0, ptr %args.i.i, align 8, !noalias !ID
  %51 = getelementptr inbounds nuw i8, ptr %args.i.i, i64 8
  store i64 %fuel.1, ptr %51, align 8, !noalias !ID
  %52 = getelementptr inbounds nuw i8, ptr %args.i.i, i64 16
  store i64 %deadline_ms.0, ptr %52, align 8, !noalias !ID
  %53 = getelementptr inbounds nuw i8, ptr %args.i.i, i64 24
  store i64 %deadline_ms.1, ptr %53, align 8, !noalias !ID
  %54 = getelementptr inbounds nuw i8, ptr %args.i.i, i64 32
  store i64 %max_answers.0, ptr %54, align 8, !noalias !ID
  %55 = getelementptr inbounds nuw i8, ptr %args.i.i, i64 40
  store i64 %max_answers.1, ptr %55, align 8, !noalias !ID
  %56 = getelementptr inbounds nuw i8, ptr %args.i.i, i64 48
  store i64 %max_intermediate_cells.0, ptr %56, align 8, !noalias !ID
  %57 = getelementptr inbounds nuw i8, ptr %args.i.i, i64 56
  store i64 %max_intermediate_cells.1, ptr %57, align 8, !noalias !ID
  %58 = getelementptr inbounds nuw i8, ptr %args.i.i, i64 64
  store i64 %max_scratch_bytes.0, ptr %58, align 8, !noalias !ID
  %59 = getelementptr inbounds nuw i8, ptr %args.i.i, i64 72
  store i64 %max_scratch_bytes.1, ptr %59, align 8, !noalias !ID
  %60 = getelementptr inbounds nuw i8, ptr %args.i.i, i64 80
  store i64 %max_remote_requests.0, ptr %60, align 8, !noalias !ID
  %61 = getelementptr inbounds nuw i8, ptr %args.i.i, i64 88
  store i64 %max_remote_requests.1, ptr %61, align 8, !noalias !ID
  %62 = getelementptr inbounds nuw i8, ptr %args.i.i, i64 96
  store i8 %22, ptr %62, align 8, !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %_39.sroa.6.i.i)
  call void @llvm.lifetime.start.p0(ptr nonnull %_40.sroa.8.i.i)
  call void @llvm.lifetime.start.p0(ptr nonnull %_40.sroa.12.i.i)
  %63 = getelementptr inbounds nuw i8, ptr %_42.i.i, i64 248
  store ptr %self, ptr %63, align 8, !noalias !ID
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %_42.i.i, ptr noundef nonnull align 8 dereferenceable(24) %specs.i.i, i64 24, i1 false), !noalias !ID
  %64 = getelementptr inbounds nuw i8, ptr %_42.i.i, i64 48
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %64, ptr noundef nonnull readonly align 8 dereferenceable(24) %25, i64 24, i1 false), !noalias !ID
  %65 = getelementptr inbounds nuw i8, ptr %_42.i.i, i64 72
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(96) %65, ptr noundef nonnull align 8 dereferenceable(96) %config.sroa.0.i.i, i64 96, i1 false), !noalias !ID
  %config.sroa.8.0..sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %_42.i.i, i64 168
  store i8 %46, ptr %config.sroa.8.0..sroa_idx.i.i, align 8, !noalias !ID
  %66 = getelementptr inbounds nuw i8, ptr %_42.i.i, i64 256
  store ptr %query.0, ptr %66, align 8, !noalias !ID
  %67 = getelementptr inbounds nuw i8, ptr %_42.i.i, i64 264
  store i64 %query.1, ptr %67, align 8, !noalias !ID
  %68 = getelementptr inbounds nuw i8, ptr %_42.i.i, i64 24
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %68, ptr noundef nonnull align 8 dereferenceable(24) %subs.i.i, i64 24, i1 false), !noalias !ID
  %69 = getelementptr inbounds nuw i8, ptr %_42.i.i, i64 240
  store i64 %_61.sroa.0.0.copyload.i.i, ptr %69, align 8, !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %_7.sroa.0.i.i.i)
  call void @llvm.lifetime.start.p0(ptr nonnull %_8.i.i.i), !noalias !ID
; invoke <purrdf_native::py_store::query::GovernorArgs>::engage
  invoke fastcc void @<purrdf_native::py_store::query::GovernorArgs>::engage(ptr noalias nofree noundef align 8 captures(none) dereferenceable(88) %_8.i.i.i, ptr noalias nofree noundef nonnull readonly align 8 captures(none) dereferenceable(104) %args.i.i, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable_or_null(8) %cancel)
          to label %bb1.i.i.i unwind label %bb17.i.i.i, !noalias !ID

bb1.i.i.i:                                        ; preds = %bb58.i.i
  %70 = getelementptr inbounds nuw i8, ptr %_8.i.i.i, i64 80
  %71 = load ptr, ptr %70, align 8, !noalias !ID, !noundef !ID
  %72 = icmp eq ptr %71, null
  br i1 %72, label %bb19.i.i.i, label %bb20.i.i.i

bb19.i.i.i:                                       ; preds = %bb1.i.i.i
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %_7.sroa.0.i.i.i, ptr noundef nonnull align 8 dereferenceable(48) %_8.i.i.i, i64 48, i1 false), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_8.i.i.i), !noalias !ID
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %_40.sroa.8.i.i, ptr noundef nonnull align 8 dereferenceable(48) %_7.sroa.0.i.i.i, i64 48, i1 false), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_7.sroa.0.i.i.i)
; invoke core::ptr::drop_glue::<<purrdf_native::py_store::quad_store::PyQuadStore>::query_impl<()>::{closure#0}::{closure#0}>
  invoke fastcc void @core::ptr::drop_glue::<<purrdf_native::py_store::quad_store::PyQuadStore>::query_impl<()>::{closure#0}::{closure#0}>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(272) %_42.i.i)
          to label %bb6.thread.i.i unwind label %cleanup9.i.i, !noalias !ID

bb20.i.i.i:                                       ; preds = %bb1.i.i.i
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(80) %_7.sroa.0.i.i.i, ptr noundef nonnull align 8 dereferenceable(80) %_8.i.i.i, i64 80, i1 false), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_8.i.i.i), !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %governors.i.i.i), !noalias !ID
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(80) %governors.i.i.i, ptr noundef nonnull align 8 dereferenceable(80) %_7.sroa.0.i.i.i, i64 80, i1 false), !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %watch.i.i.i), !noalias !ID
  store ptr %71, ptr %watch.i.i.i, align 8, !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_7.sroa.0.i.i.i)
  call void @llvm.lifetime.start.p0(ptr nonnull %outcome.i.i.i), !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %_11.i.i.i), !noalias !ID
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(272) %_11.i.i.i, ptr noundef nonnull readonly align 8 dereferenceable(272) %_42.i.i, i64 272, i1 false), !noalias !ID
  %73 = getelementptr inbounds nuw i8, ptr %_11.i.i.i, i64 272
  store ptr %governors.i.i.i, ptr %73, align 8, !noalias !ID
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  call void @llvm.lifetime.start.p0(ptr nonnull %_guard.i.i.i.i), !noalias !ID
; invoke <pyo3::internal::state::SuspendAttach>::new
  %74 = invoke { i64, ptr } @<pyo3::internal::state::SuspendAttach>::new()
          to label %bb1.i.i.i.i unwind label %bb6.i.i.i.i, !noalias !ID

bb1.i.i.i.i:                                      ; preds = %bb20.i.i.i
  %75 = extractvalue { i64, ptr } %74, 0
  %76 = extractvalue { i64, ptr } %74, 1
  store i64 %75, ptr %_guard.i.i.i.i, align 8, !noalias !ID
  %77 = getelementptr inbounds nuw i8, ptr %_guard.i.i.i.i, i64 8
  store ptr %76, ptr %77, align 8, !noalias !ID
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  call void @llvm.lifetime.start.p0(ptr nonnull %parser_options.i.i.i.i.i.i), !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %_27.i.i.i.i.i.i), !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %dataset.i.i.i.i.i.i), !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %_6.i.i.i.i.i.i), !noalias !ID
  %78 = getelementptr inbounds nuw i8, ptr %_11.i.i.i, i64 248
  %_38.i.i.i.i.i.i = load ptr, ptr %78, align 8, !alias.scope !ID, !noalias !ID, !nonnull !ID, !align !ID, !noundef !ID
; invoke <purrdf_core::ir::mutable::MutableDataset>::freeze
  invoke void @<purrdf_core::ir::mutable::MutableDataset>::freeze(ptr noalias nofree noundef nonnull sret([96 x i8]) align 8 captures(none) dereferenceable(96) %_6.i.i.i.i.i.i, ptr noundef nonnull align 8 %_38.i.i.i.i.i.i)
          to label %bb1.i.i.i.i.i.i unwind label %bb41.thread120.i.i.i.i.i.i, !noalias !ID

bb41.thread120.i.i.i.i.i.i:                       ; preds = %bb8.i.i.i.i.i.i.i, %bb1.i.i.i.i
  %lpad.thr_comm.i.i.i.i.i.i = landingpad { ptr, i32 }
          cleanup
  br label %bb40.i.i.i.i.i.i

bb41.i.i.i.i.i.i:                                 ; preds = %bb2.i.i91.i.i.i.i.i.i, %bb2.i.i69.i.i.i.i.i.i
  %_40.sroa.0.1.ph.i.i.i.i.i.i = phi i8 [ %_40.sroa.0.7.i.i.i.i.i.i, %bb2.i.i91.i.i.i.i.i.i ], [ 0, %bb2.i.i69.i.i.i.i.i.i ]
  %lpad.thr_comm.split-lp.i.i.i.i.i.i = landingpad { ptr, i32 }
          cleanup
  br label %bb39.i.i.i.i.i.i

bb1.i.i.i.i.i.i:                                  ; preds = %bb1.i.i.i.i
  %79 = load i64, ptr %_6.i.i.i.i.i.i, align 8, !range !ID, !noalias !ID, !noundef !ID
  %.not.i.i.i.i.i.i = icmp eq i64 %79, -1
  br i1 %.not.i.i.i.i.i.i, label %bb43.i.i.i.i.i.i, label %bb42.i.i.i.i.i.i

bb42.i.i.i.i.i.i:                                 ; preds = %bb1.i.i.i.i.i.i
  call void @llvm.lifetime.start.p0(ptr nonnull %_47.i.i.i.i.i.i), !noalias !ID
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(96) %_47.i.i.i.i.i.i, ptr noundef nonnull align 8 dereferenceable(96) %_6.i.i.i.i.i.i, i64 96, i1 false), !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %_3.i.i.i.i.i.i.i), !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %args.i.i.i.i.i.i.i), !noalias !ID
  store ptr %_47.i.i.i.i.i.i, ptr %args.i.i.i.i.i.i.i, align 8, !noalias !ID
  %_7.sroa.4.0..sroa_idx.i.i.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %args.i.i.i.i.i.i.i, i64 8
  store ptr @<purrdf_core::diagnostic::RdfDiagnostic as core::fmt::Display>::fmt, ptr %_7.sroa.4.0..sroa_idx.i.i.i.i.i.i.i, align 8, !noalias !ID
; invoke alloc::fmt::format::format_inner
  invoke void @alloc::fmt::format::format_inner(ptr noalias nofree noundef nonnull sret([24 x i8]) align 8 captures(none) dereferenceable(24) %_3.i.i.i.i.i.i.i, ptr noundef nonnull @alloc_0e5f90e3dc675d538220deef7d17e145, ptr noundef nonnull %args.i.i.i.i.i.i.i)
          to label %bb4.i.i.i.i.i.i.i unwind label %cleanup.i.i.i.i.i.i.i, !noalias !ID

cleanup.i.i.i.i.i.i.i:                            ; preds = %bb42.i.i.i.i.i.i
  %80 = landingpad { ptr, i32 }
          cleanup
  br label %cleanup.body.i.i.i.i.i.i.i

cleanup.body.i.i.i.i.i.i.i:                       ; preds = %bb2.i.i.i4.i.i.i.i.i.i.i.i.i.i, %cleanup.i.i.i.i.i.i.i.i, %cleanup.i.i.i.i.i.i.i
  %eh.lpad-body.i.i.i.i.i.i.i = phi { ptr, i32 } [ %80, %cleanup.i.i.i.i.i.i.i ], [ %83, %bb2.i.i.i4.i.i.i.i.i.i.i.i.i.i ], [ %83, %cleanup.i.i.i.i.i.i.i.i ]
; invoke core::ptr::drop_glue::<purrdf_core::diagnostic::RdfDiagnostic>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_core::diagnostic::RdfDiagnostic>(ptr noalias nofree noundef nonnull align 8 dereferenceable(96) %_47.i.i.i.i.i.i) #ATTR
          to label %bb40.i.i.i.i.i.i unwind label %terminate.i.i.i.i.i.i.i, !noalias !ID

bb4.i.i.i.i.i.i.i:                                ; preds = %bb42.i.i.i.i.i.i
  call void @llvm.lifetime.end.p0(ptr nonnull %args.i.i.i.i.i.i.i), !noalias !ID
  %_29.sroa.0.0.copyload.i.i.i.i.i.i.i = load i64, ptr %_3.i.i.i.i.i.i.i, align 8, !noalias !ID
  %_29.sroa.5.0._3.sroa_idx.i.i.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %_3.i.i.i.i.i.i.i, i64 8
  %_29.sroa.5.0.copyload.i.i.i.i.i.i.i = load ptr, ptr %_29.sroa.5.0._3.sroa_idx.i.i.i.i.i.i.i, align 8, !noalias !ID
  %_29.sroa.6.0._3.sroa_idx.i.i.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %_3.i.i.i.i.i.i.i, i64 16
  %_29.sroa.6.0.copyload.i.i.i.i.i.i.i = load i64, ptr %_29.sroa.6.0._3.sroa_idx.i.i.i.i.i.i.i, align 8, !noalias !ID
; call __rustc::__rust_no_alloc_shim_is_unstable_v2
  call void @__rustc::__rust_no_alloc_shim_is_unstable_v2() #ATTR, !noalias !ID
; call __rustc::__rust_alloc
  %81 = call noundef align 8 dereferenceable_or_null(24) ptr @__rustc::__rust_alloc(i64 noundef 24, i64 noundef range(i64 1, -9223372036854775807) 8) #ATTR, !noalias !ID
  %82 = icmp eq ptr %81, null
  br i1 %82, label %bb2.i5.i.i.i.i.i.i.i, label %bb8.i.i.i.i.i.i.i, !prof !ID

bb2.i5.i.i.i.i.i.i.i:                             ; preds = %bb4.i.i.i.i.i.i.i
; invoke alloc::alloc::handle_alloc_error
  invoke void @alloc::alloc::handle_alloc_error(i64 noundef 8, i64 noundef 24) #ATTR
          to label %.noexc.i.i.i.i.i.i.i unwind label %cleanup.i.i.i.i.i.i.i.i, !noalias !ID

.noexc.i.i.i.i.i.i.i:                             ; preds = %bb2.i5.i.i.i.i.i.i.i
  unreachable

cleanup.i.i.i.i.i.i.i.i:                          ; preds = %bb2.i5.i.i.i.i.i.i.i
  %83 = landingpad { ptr, i32 }
          cleanup
  %84 = icmp eq i64 %_29.sroa.0.0.copyload.i.i.i.i.i.i.i, 0
  br i1 %84, label %cleanup.body.i.i.i.i.i.i.i, label %bb2.i.i.i4.i.i.i.i.i.i.i.i.i.i

bb2.i.i.i4.i.i.i.i.i.i.i.i.i.i:                   ; preds = %cleanup.i.i.i.i.i.i.i.i
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %_29.sroa.5.0.copyload.i.i.i.i.i.i.i) ]
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %_29.sroa.5.0.copyload.i.i.i.i.i.i.i, i64 noundef %_29.sroa.0.0.copyload.i.i.i.i.i.i.i, i64 noundef range(i64 1, -9223372036854775807) 1) #ATTR, !noalias !ID
  br label %cleanup.body.i.i.i.i.i.i.i

bb8.i.i.i.i.i.i.i:                                ; preds = %bb4.i.i.i.i.i.i.i
  store i64 %_29.sroa.0.0.copyload.i.i.i.i.i.i.i, ptr %81, align 8, !noalias !ID
  %_29.sroa.5.0..sroa_idx.i.i.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %81, i64 8
  store ptr %_29.sroa.5.0.copyload.i.i.i.i.i.i.i, ptr %_29.sroa.5.0..sroa_idx.i.i.i.i.i.i.i, align 8, !noalias !ID
  %_29.sroa.6.0..sroa_idx.i.i.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %81, i64 16
  store i64 %_29.sroa.6.0.copyload.i.i.i.i.i.i.i, ptr %_29.sroa.6.0..sroa_idx.i.i.i.i.i.i.i, align 8, !noalias !ID
; invoke core::ptr::drop_glue::<purrdf_core::diagnostic::RdfDiagnostic>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_core::diagnostic::RdfDiagnostic>(ptr noalias nofree noundef nonnull align 8 dereferenceable(96) %_47.i.i.i.i.i.i)
          to label %bb44.i.i.i.i.i.i unwind label %bb41.thread120.i.i.i.i.i.i, !noalias !ID

terminate.i.i.i.i.i.i.i:                          ; preds = %cleanup.body.i.i.i.i.i.i.i
  %85 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #ATTR, !noalias !ID
  unreachable

bb43.i.i.i.i.i.i:                                 ; preds = %bb1.i.i.i.i.i.i
  %86 = getelementptr inbounds nuw i8, ptr %_6.i.i.i.i.i.i, i64 8
  %_44.i.i.i.i.i.i = load ptr, ptr %86, align 8, !noalias !ID, !nonnull !ID, !noundef !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_6.i.i.i.i.i.i), !noalias !ID
  store ptr %_44.i.i.i.i.i.i, ptr %dataset.i.i.i.i.i.i, align 8, !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %registry.i.i.i.i.i.i), !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %_10.sroa.5.i.i.i.i.i.i)
  call void @llvm.lifetime.start.p0(ptr nonnull %_11.i.i.i.i.i.i), !noalias !ID
  %_13.i.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %_44.i.i.i.i.i.i, i64 16
; invoke purrdf_native::py_store::query::build_relations
  invoke fastcc void @purrdf_native::py_store::query::build_relations(ptr noalias nofree noundef align 8 captures(none) dereferenceable(80) %_11.i.i.i.i.i.i, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(280) %_11.i.i.i, ptr noundef nonnull align 8 %_13.i.i.i.i.i.i)
          to label %bb3.i.i.i.i.i.i unwind label %cleanup5.i.i.i.i.i.i, !noalias !ID

bb22.i.i.i.i.i.i:                                 ; preds = %cleanup.i.i79.i.i.i.i.i.i, %cleanup.i.i62.i.i.i.i.i.i, %bb21.i.i.i.i.i.i, %cleanup5.i.i.i.i.i.i
  %_40.sroa.0.2.i.i.i.i.i.i = phi i8 [ %_40.sroa.0.4.i.i.i.i.i.i, %bb21.i.i.i.i.i.i ], [ 0, %cleanup.i.i62.i.i.i.i.i.i ], [ %_40.sroa.0.3.i.i.i.i.i.i, %cleanup5.i.i.i.i.i.i ], [ 0, %cleanup.i.i79.i.i.i.i.i.i ]
  %.pn30.i.i.i.i.i.i = phi { ptr, i32 } [ %.pn28.i.i.i.i.i.i, %bb21.i.i.i.i.i.i ], [ %145, %cleanup.i.i62.i.i.i.i.i.i ], [ %88, %cleanup5.i.i.i.i.i.i ], [ %152, %cleanup.i.i79.i.i.i.i.i.i ]
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %_10.i.i.i.i.i.i.i.i = load ptr, ptr %dataset.i.i.i.i.i.i, align 8, !alias.scope !ID, !noalias !ID, !nonnull !ID, !noundef !ID
  %_2.i.i.i.i.i.i.i.i = atomicrmw sub ptr %_10.i.i.i.i.i.i.i.i, i64 1 release, align 8, !noalias !ID
  %87 = icmp eq i64 %_2.i.i.i.i.i.i.i.i, 1
  br i1 %87, label %bb2.i.i.i.i.i.i.i.i, label %bb39.i.i.i.i.i.i

bb2.i.i.i.i.i.i.i.i:                              ; preds = %bb22.i.i.i.i.i.i
  fence acquire
; invoke <alloc::sync::Arc<purrdf_core::ir::dataset::RdfDataset>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_core::ir::dataset::RdfDataset>>::drop_slow(ptr noalias nofree noundef nonnull align 8 dereferenceable(8) %dataset.i.i.i.i.i.i) #ATTR
          to label %bb39.i.i.i.i.i.i unwind label %terminate.i.i.i.i.i.i, !noalias !ID

cleanup5.i.i.i.i.i.i:                             ; preds = %core::ptr::drop_glue::<purrdf_sparql_eval::property_fn::PropertyFunctionRegistry> (.exit.i82.i.i.i.i.i.i), %core::ptr::drop_glue::<purrdf_sparql_eval::property_fn::PropertyFunctionRegistry> (.exit.i.i.i.i.i.i.i), %bb43.i.i.i.i.i.i
  %_40.sroa.0.3.i.i.i.i.i.i = phi i8 [ 0, %core::ptr::drop_glue::<purrdf_sparql_eval::property_fn::PropertyFunctionRegistry> (.exit.i82.i.i.i.i.i.i) ], [ 0, %core::ptr::drop_glue::<purrdf_sparql_eval::property_fn::PropertyFunctionRegistry> (.exit.i.i.i.i.i.i.i) ], [ 1, %bb43.i.i.i.i.i.i ]
  %88 = landingpad { ptr, i32 }
          cleanup
  br label %bb22.i.i.i.i.i.i

bb3.i.i.i.i.i.i:                                  ; preds = %bb43.i.i.i.i.i.i
  %_56.i.i.i.i.i.i = load i64, ptr %_11.i.i.i.i.i.i, align 8, !range !ID, !noalias !ID, !noundef !ID
  %89 = trunc nuw i64 %_56.i.i.i.i.i.i to i1
  %90 = getelementptr inbounds nuw i8, ptr %_11.i.i.i.i.i.i, i64 8
  br i1 %89, label %bb45.i.i.i.i.i.i, label %bb46.i.i.i.i.i.i

bb45.i.i.i.i.i.i:                                 ; preds = %bb3.i.i.i.i.i.i
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %_10.sroa.5.i.i.i.i.i.i, ptr noundef nonnull align 8 dereferenceable(48) %90, i64 48, i1 false), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_11.i.i.i.i.i.i), !noalias !ID
  %91 = getelementptr inbounds nuw i8, ptr %outcome.i.i.i, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %91, ptr noundef nonnull align 8 dereferenceable(48) %_10.sroa.5.i.i.i.i.i.i, i64 48, i1 false), !noalias !ID
  store i64 -2, ptr %outcome.i.i.i, align 8, !alias.scope !ID, !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_10.sroa.5.i.i.i.i.i.i)
  br label %bb16.i.i.i.i.i.i

bb46.i.i.i.i.i.i:                                 ; preds = %bb3.i.i.i.i.i.i
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %_10.sroa.5.i.i.i.i.i.i, ptr noundef nonnull align 8 dereferenceable(72) %90, i64 72, i1 false), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_11.i.i.i.i.i.i), !noalias !ID
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %registry.i.i.i.i.i.i, ptr noundef nonnull align 8 dereferenceable(72) %_10.sroa.5.i.i.i.i.i.i, i64 72, i1 false), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_10.sroa.5.i.i.i.i.i.i)
  call void @llvm.lifetime.start.p0(ptr nonnull %aggregates.i.i.i.i.i.i), !noalias !ID
  %92 = getelementptr inbounds nuw i8, ptr %_11.i.i.i, i64 48
  %93 = load i64, ptr %92, align 8, !range !ID, !alias.scope !ID, !noalias !ID, !noundef !ID
  %.not21.i.i.i.i.i.i = icmp eq i64 %93, -1
  %94 = getelementptr inbounds nuw i8, ptr %_11.i.i.i, i64 56
  %_67.i.i.i.i.i.i = load ptr, ptr %94, align 8, !alias.scope !ID, !noalias !ID, !nonnull !ID
  %95 = getelementptr inbounds nuw i8, ptr %_11.i.i.i, i64 64
  %_66.i.i.i.i.i.i = load i64, ptr %95, align 8, !alias.scope !ID, !noalias !ID
  %_17.sroa.5.0.i.i.i.i.i.i = select i1 %.not21.i.i.i.i.i.i, i64 undef, i64 %_66.i.i.i.i.i.i
  %_17.sroa.0.0.i.i.i.i.i.i = select i1 %.not21.i.i.i.i.i.i, ptr null, ptr %_67.i.i.i.i.i.i
; invoke purrdf_validate::query::statistical_aggregates
  invoke void @purrdf_validate::query::statistical_aggregates(ptr noalias nofree noundef nonnull sret([40 x i8]) align 8 captures(none) dereferenceable(40) %aggregates.i.i.i.i.i.i, ptr noalias nofree noundef readonly captures(address, read_provenance) %_17.sroa.0.0.i.i.i.i.i.i, i64 %_17.sroa.5.0.i.i.i.i.i.i)
          to label %bb4.i.i.i.i.i.i unwind label %cleanup6.i.i.i.i.i.i, !noalias !ID

bb21.i.i.i.i.i.i:                                 ; preds = %bb2.i.i.i.i.i.i.i, %bb20.i.i.i.i.i.i, %cleanup6.i.i.i.i.i.i
  %_40.sroa.0.4.i.i.i.i.i.i = phi i8 [ %_40.sroa.0.5.i.i.i.i.i.i, %cleanup6.i.i.i.i.i.i ], [ %_40.sroa.0.6.i.i.i.i.i.i, %bb2.i.i.i.i.i.i.i ], [ %_40.sroa.0.6.i.i.i.i.i.i, %bb20.i.i.i.i.i.i ]
  %.pn28.i.i.i.i.i.i = phi { ptr, i32 } [ %96, %cleanup6.i.i.i.i.i.i ], [ %.pn25.pn.i.i.i.i.i.i, %bb2.i.i.i.i.i.i.i ], [ %.pn25.pn.i.i.i.i.i.i, %bb20.i.i.i.i.i.i ]
; invoke core::ptr::drop_glue::<core::option::Option<purrdf_sparql_eval::property_fn::PropertyFunctionRegistry>>
  invoke fastcc void @core::ptr::drop_glue::<core::option::Option<purrdf_sparql_eval::property_fn::PropertyFunctionRegistry>>(ptr noalias nofree noundef align 8 dereferenceable(72) %registry.i.i.i.i.i.i) #ATTR
          to label %bb22.i.i.i.i.i.i unwind label %terminate.i.i.i.i.i.i, !noalias !ID

cleanup6.i.i.i.i.i.i:                             ; preds = %bb2.i74.i.i.i.i.i.i, %bb2.i57.i.i.i.i.i.i, %bb46.i.i.i.i.i.i
  %_40.sroa.0.5.i.i.i.i.i.i = phi i8 [ 0, %bb2.i74.i.i.i.i.i.i ], [ 0, %bb2.i57.i.i.i.i.i.i ], [ 1, %bb46.i.i.i.i.i.i ]
  %96 = landingpad { ptr, i32 }
          cleanup
  br label %bb21.i.i.i.i.i.i

bb4.i.i.i.i.i.i:                                  ; preds = %bb46.i.i.i.i.i.i
  %_19.i.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %_11.i.i.i, i64 72
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  call void @llvm.lifetime.start.p0(ptr nonnull %_2.i.i.i.i.i.i.i), !noalias !ID
  %97 = load i64, ptr %_19.i.i.i.i.i.i, align 8, !range !ID, !alias.scope !ID, !noalias !ID, !noundef !ID
  %.not.i.i.i.i.i.i.i = icmp eq i64 %97, -1
  br i1 %.not.i.i.i.i.i.i.i, label %bb4.i40.i.i.i.i.i.i, label %bb5.i.i.i.i.i.i.i

bb5.i.i.i.i.i.i.i:                                ; preds = %bb4.i.i.i.i.i.i
  %98 = getelementptr inbounds nuw i8, ptr %_11.i.i.i, i64 80
  %config.val.i.i.i.i.i.i.i = load ptr, ptr %98, align 8, !alias.scope !ID, !noalias !ID, !nonnull !ID, !noundef !ID
  %99 = getelementptr inbounds nuw i8, ptr %_11.i.i.i, i64 88
  %config.val6.i.i.i.i.i.i.i = load i64, ptr %99, align 8, !alias.scope !ID, !noalias !ID, !noundef !ID
; invoke <alloc::vec::Vec<alloc::string::String> as core::clone::Clone>::clone
  invoke fastcc void @<alloc::vec::Vec<alloc::string::String> as core::clone::Clone>::clone(ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %_2.i.i.i.i.i.i.i, ptr nonnull %config.val.i.i.i.i.i.i.i, i64 %config.val6.i.i.i.i.i.i.i)
          to label %bb7.i.i.i.i.i.i.i unwind label %cleanup7.i.i.i.i.i.i, !noalias !ID

bb4.i40.i.i.i.i.i.i:                              ; preds = %bb4.i.i.i.i.i.i
  store i64 0, ptr %_2.i.i.i.i.i.i.i, align 8, !noalias !ID
  %100 = getelementptr inbounds nuw i8, ptr %_2.i.i.i.i.i.i.i, i64 8
  store ptr inttoptr (i64 8 to ptr), ptr %100, align 8, !noalias !ID
  %101 = getelementptr inbounds nuw i8, ptr %_2.i.i.i.i.i.i.i, i64 16
  store i64 0, ptr %101, align 8, !noalias !ID
  br label %bb7.i.i.i.i.i.i.i

bb7.i.i.i.i.i.i.i:                                ; preds = %bb4.i40.i.i.i.i.i.i, %bb5.i.i.i.i.i.i.i
  %102 = getelementptr inbounds nuw i8, ptr %_11.i.i.i, i64 96
  %103 = load i64, ptr %102, align 8, !range !ID, !alias.scope !ID, !noalias !ID, !noundef !ID
  %.not4.i.i.i.i.i.i.i = icmp eq i64 %103, -1
  br i1 %.not4.i.i.i.i.i.i.i, label %bb5.i.i.i.i.i.i, label %bb9.i.i.i.i.i.i.i

bb9.i.i.i.i.i.i.i:                                ; preds = %bb7.i.i.i.i.i.i.i
  call void @llvm.lifetime.start.p0(ptr nonnull %_12.i.i.i.i.i.i.i), !noalias !ID
  %104 = getelementptr inbounds nuw i8, ptr %_11.i.i.i, i64 104
  %.val.i.i.i.i.i.i.i = load ptr, ptr %104, align 8, !alias.scope !ID, !noalias !ID, !nonnull !ID, !noundef !ID
  %105 = getelementptr inbounds nuw i8, ptr %_11.i.i.i, i64 112
  %.val5.i.i.i.i.i.i.i = load i64, ptr %105, align 8, !alias.scope !ID, !noalias !ID, !noundef !ID
; invoke <alloc::vec::Vec<alloc::string::String> as core::clone::Clone>::clone
  invoke fastcc void @<alloc::vec::Vec<alloc::string::String> as core::clone::Clone>::clone(ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %_12.i.i.i.i.i.i.i, ptr nonnull %.val.i.i.i.i.i.i.i, i64 %.val5.i.i.i.i.i.i.i)
          to label %bb10.i.i.i.i.i.i.i unwind label %cleanup.i39.i.i.i.i.i.i, !noalias !ID

cleanup.i39.i.i.i.i.i.i:                          ; preds = %bb9.i.i.i.i.i.i.i
  %106 = landingpad { ptr, i32 }
          cleanup
; call core::ptr::drop_glue::<alloc::vec::Vec<alloc::string::String>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<alloc::string::String>>(ptr noalias nofree noundef align 8 dereferenceable(24) %_2.i.i.i.i.i.i.i) #ATTR, !noalias !ID
  br label %bb20.i.i.i.i.i.i

bb10.i.i.i.i.i.i.i:                               ; preds = %bb9.i.i.i.i.i.i.i
  %_5.sroa.0.0.copyload.i.i.i.i.i.i.i = load i64, ptr %_12.i.i.i.i.i.i.i, align 8, !noalias !ID
  %_5.sroa.4.0._12.sroa_idx.i.i.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %_12.i.i.i.i.i.i.i, i64 8
  %_5.sroa.4.0.copyload.i.i.i.i.i.i.i = load ptr, ptr %_5.sroa.4.0._12.sroa_idx.i.i.i.i.i.i.i, align 8, !noalias !ID
  %_5.sroa.5.0._12.sroa_idx.i.i.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %_12.i.i.i.i.i.i.i, i64 16
  %_5.sroa.5.0.copyload.i.i.i.i.i.i.i = load i64, ptr %_5.sroa.5.0._12.sroa_idx.i.i.i.i.i.i.i, align 8, !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_12.i.i.i.i.i.i.i), !noalias !ID
  br label %bb5.i.i.i.i.i.i

bb20.i.i.i.i.i.i:                                 ; preds = %bb34.i.i.i.i.i.i, %bb19.i.i.i.i.i.i, %bb35.thread128.i.i.i.i.i.i, %cleanup7.i.i.i.i.i.i, %cleanup.i39.i.i.i.i.i.i
  %_40.sroa.0.6.i.i.i.i.i.i = phi i8 [ 0, %bb34.i.i.i.i.i.i ], [ 0, %bb35.thread128.i.i.i.i.i.i ], [ 1, %cleanup.i39.i.i.i.i.i.i ], [ 1, %cleanup7.i.i.i.i.i.i ], [ 0, %bb19.i.i.i.i.i.i ]
  %.pn25.pn.i.i.i.i.i.i = phi { ptr, i32 } [ %lpad.thr_comm.split-lp127.i.i.i.i.i.i, %bb34.i.i.i.i.i.i ], [ %lpad.thr_comm126.i.i.i.i.i.i, %bb35.thread128.i.i.i.i.i.i ], [ %106, %cleanup.i39.i.i.i.i.i.i ], [ %109, %cleanup7.i.i.i.i.i.i ], [ %.pn.i.i.i.i.i.i, %bb19.i.i.i.i.i.i ]
  %107 = load ptr, ptr %aggregates.i.i.i.i.i.i, align 8, !alias.scope !ID, !noalias !ID, !noundef !ID
  %108 = icmp eq ptr %107, null
  br i1 %108, label %bb21.i.i.i.i.i.i, label %bb2.i.i.i.i.i.i.i

bb2.i.i.i.i.i.i.i:                                ; preds = %bb20.i.i.i.i.i.i
; invoke core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, alloc::sync::Arc<dyn purrdf_sparql_eval::agg_fn::CustomAggregate>)>>
  invoke fastcc void @core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, alloc::sync::Arc<dyn purrdf_sparql_eval::agg_fn::CustomAggregate>)>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(40) %aggregates.i.i.i.i.i.i)
          to label %bb21.i.i.i.i.i.i unwind label %terminate.i.i.i.i.i.i, !noalias !ID

cleanup7.i.i.i.i.i.i:                             ; preds = %bb5.i.i.i.i.i.i.i
  %109 = landingpad { ptr, i32 }
          cleanup
  br label %bb20.i.i.i.i.i.i

bb5.i.i.i.i.i.i:                                  ; preds = %bb10.i.i.i.i.i.i.i, %bb7.i.i.i.i.i.i.i
  %_4.sroa.6.0.i.i.i.i.i.i.i = phi i64 [ %_5.sroa.5.0.copyload.i.i.i.i.i.i.i, %bb10.i.i.i.i.i.i.i ], [ 0, %bb7.i.i.i.i.i.i.i ]
  %_4.sroa.5.0.i.i.i.i.i.i.i = phi ptr [ %_5.sroa.4.0.copyload.i.i.i.i.i.i.i, %bb10.i.i.i.i.i.i.i ], [ inttoptr (i64 8 to ptr), %bb7.i.i.i.i.i.i.i ]
  %_4.sroa.0.0.i.i.i.i.i.i.i = phi i64 [ %_5.sroa.0.0.copyload.i.i.i.i.i.i.i, %bb10.i.i.i.i.i.i.i ], [ 0, %bb7.i.i.i.i.i.i.i ]
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %parser_options.i.i.i.i.i.i, ptr noundef nonnull align 8 dereferenceable(24) %_2.i.i.i.i.i.i.i, i64 24, i1 false), !noalias !ID
  %110 = getelementptr inbounds nuw i8, ptr %parser_options.i.i.i.i.i.i, i64 24
  store i64 %_4.sroa.0.0.i.i.i.i.i.i.i, ptr %110, align 8, !noalias !ID
  %_4.sroa.5.0..sroa_idx.i.i.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %parser_options.i.i.i.i.i.i, i64 32
  store ptr %_4.sroa.5.0.i.i.i.i.i.i.i, ptr %_4.sroa.5.0..sroa_idx.i.i.i.i.i.i.i, align 8, !noalias !ID
  %_4.sroa.6.0..sroa_idx.i.i.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %parser_options.i.i.i.i.i.i, i64 40
  store i64 %_4.sroa.6.0.i.i.i.i.i.i.i, ptr %_4.sroa.6.0..sroa_idx.i.i.i.i.i.i.i, align 8, !noalias !ID
  %111 = getelementptr inbounds nuw i8, ptr %parser_options.i.i.i.i.i.i, i64 48
  store i64 0, ptr %111, align 8, !noalias !ID
  %_6.sroa.4.0..sroa_idx.i.i.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %parser_options.i.i.i.i.i.i, i64 56
  store ptr inttoptr (i64 8 to ptr), ptr %_6.sroa.4.0..sroa_idx.i.i.i.i.i.i.i, align 8, !noalias !ID
  %_6.sroa.5.0..sroa_idx.i.i.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %parser_options.i.i.i.i.i.i, i64 64
  store i64 0, ptr %_6.sroa.5.0..sroa_idx.i.i.i.i.i.i.i, align 8, !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_2.i.i.i.i.i.i.i), !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %engine.i.i.i.i.i.i), !noalias !ID
; invoke purrdf_native::py_store::query::build_engine
  invoke fastcc void @purrdf_native::py_store::query::build_engine(ptr noalias nofree noundef align 8 captures(address) dereferenceable(520) %engine.i.i.i.i.i.i, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(168) %_19.i.i.i.i.i.i)
          to label %bb6.i.i.i.i.i.i unwind label %bb34.i.i.i.i.i.i, !noalias !ID

bb35.thread128.i.i.i.i.i.i:                       ; preds = %bb9.i.i.i.i.i.i, %bb56.i.i.i.i.i.i
  %lpad.thr_comm126.i.i.i.i.i.i = landingpad { ptr, i32 }
          cleanup
  br label %bb20.i.i.i.i.i.i

bb6.i.i.i.i.i.i:                                  ; preds = %bb5.i.i.i.i.i.i
  call void @llvm.lifetime.start.p0(ptr nonnull %_22.i.i.i.i.i.i), !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %_25.i.i.i.i.i.i), !noalias !ID
  %112 = getelementptr inbounds nuw i8, ptr %_11.i.i.i, i64 256
  %_39.0.i.i.i.i.i.i = load ptr, ptr %112, align 8, !alias.scope !ID, !noalias !ID, !nonnull !ID, !noundef !ID
  %113 = getelementptr inbounds nuw i8, ptr %_11.i.i.i, i64 264
  %_39.1.i.i.i.i.i.i = load i64, ptr %113, align 8, !alias.scope !ID, !noalias !ID, !noundef !ID
  %114 = getelementptr inbounds nuw i8, ptr %_11.i.i.i, i64 32
  %_74.i.i.i.i.i.i = load ptr, ptr %114, align 8, !alias.scope !ID, !noalias !ID, !nonnull !ID, !noundef !ID
  %115 = getelementptr inbounds nuw i8, ptr %_11.i.i.i, i64 40
  %_73.i.i.i.i.i.i = load i64, ptr %115, align 8, !alias.scope !ID, !noalias !ID, !noundef !ID
  store ptr %_39.0.i.i.i.i.i.i, ptr %_25.i.i.i.i.i.i, align 8, !noalias !ID
  %116 = getelementptr inbounds nuw i8, ptr %_25.i.i.i.i.i.i, i64 8
  store i64 %_39.1.i.i.i.i.i.i, ptr %116, align 8, !noalias !ID
  %117 = getelementptr inbounds nuw i8, ptr %_25.i.i.i.i.i.i, i64 32
  store ptr null, ptr %117, align 8, !noalias !ID
  %118 = getelementptr inbounds nuw i8, ptr %_25.i.i.i.i.i.i, i64 16
  store ptr %_74.i.i.i.i.i.i, ptr %118, align 8, !noalias !ID
  %119 = getelementptr inbounds nuw i8, ptr %_25.i.i.i.i.i.i, i64 24
  store i64 %_73.i.i.i.i.i.i, ptr %119, align 8, !noalias !ID
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(184) %_27.i.i.i.i.i.i, ptr noundef nonnull align 8 dereferenceable(184) @anon.HASH.4, i64 184, i1 false), !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %_29.i.i.i.i.i.i), !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %_30.sroa.6.i.i.i.i.i.i)
  call void @llvm.lifetime.start.p0(ptr nonnull %_31.i.i.i.i.i.i), !noalias !ID
  %120 = load ptr, ptr %registry.i.i.i.i.i.i, align 8, !noalias !ID, !noundef !ID
  %.not22.i.i.i.i.i.i = icmp eq ptr %120, null
  %.registry.i.i.i.i.i.i = select i1 %.not22.i.i.i.i.i.i, ptr null, ptr %registry.i.i.i.i.i.i
  %121 = load ptr, ptr %aggregates.i.i.i.i.i.i, align 8, !noalias !ID, !noundef !ID
  %.not23.i.i.i.i.i.i = icmp eq ptr %121, null
  %_34.sroa.0.0.i.i.i.i.i.i = select i1 %.not23.i.i.i.i.i.i, ptr null, ptr %aggregates.i.i.i.i.i.i
; invoke purrdf_native::py_store::env::extension_env
  invoke fastcc void @purrdf_native::py_store::env::extension_env(ptr noalias nofree noundef align 8 captures(none) dereferenceable(352) %_31.i.i.i.i.i.i, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(72) %parser_options.i.i.i.i.i.i, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable_or_null(72) %.registry.i.i.i.i.i.i, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable_or_null(40) %_34.sroa.0.0.i.i.i.i.i.i)
          to label %bb7.i.i.i.i.i.i unwind label %cleanup9.i.i.i.i.i.i, !noalias !ID

bb19.i.i.i.i.i.i:                                 ; preds = %cleanup10.body.i.i.i.i.i.i, %cleanup9.i.i.i.i.i.i
  %.pn.i.i.i.i.i.i = phi { ptr, i32 } [ %122, %cleanup9.i.i.i.i.i.i ], [ %eh.lpad-body56.i.i.i.i.i.i, %cleanup10.body.i.i.i.i.i.i ]
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::engine::NativeSparqlEngine>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::engine::NativeSparqlEngine>(ptr noalias nofree noundef align 8 dereferenceable(520) %engine.i.i.i.i.i.i) #ATTR
          to label %bb20.i.i.i.i.i.i unwind label %terminate.i.i.i.i.i.i, !noalias !ID

cleanup9.i.i.i.i.i.i:                             ; preds = %bb58.i.i.i.i.i.i, %bb6.i.i.i.i.i.i
  %122 = landingpad { ptr, i32 }
          cleanup
  br label %bb19.i.i.i.i.i.i

bb7.i.i.i.i.i.i:                                  ; preds = %bb6.i.i.i.i.i.i
  %123 = load i64, ptr %_31.i.i.i.i.i.i, align 8, !range !ID, !noalias !ID, !noundef !ID
  %124 = icmp eq i64 %123, -1
  %125 = getelementptr inbounds nuw i8, ptr %_31.i.i.i.i.i.i, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %_30.sroa.6.i.i.i.i.i.i, ptr noundef nonnull align 8 dereferenceable(48) %125, i64 48, i1 false), !noalias !ID
  br i1 %124, label %bb56.i.i.i.i.i.i, label %bb57.i.i.i.i.i.i

bb56.i.i.i.i.i.i:                                 ; preds = %bb7.i.i.i.i.i.i
  call void @llvm.lifetime.end.p0(ptr nonnull %_31.i.i.i.i.i.i), !noalias !ID
  %126 = getelementptr inbounds nuw i8, ptr %outcome.i.i.i, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %126, ptr noundef nonnull align 8 dereferenceable(48) %_30.sroa.6.i.i.i.i.i.i, i64 48, i1 false), !noalias !ID
  store i64 -2, ptr %outcome.i.i.i, align 8, !alias.scope !ID, !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_25.i.i.i.i.i.i), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_30.sroa.6.i.i.i.i.i.i)
  call void @llvm.lifetime.end.p0(ptr nonnull %_29.i.i.i.i.i.i), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_22.i.i.i.i.i.i), !noalias !ID
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::engine::NativeSparqlEngine>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::engine::NativeSparqlEngine>(ptr noalias nofree noundef align 8 dereferenceable(520) %engine.i.i.i.i.i.i)
          to label %bb14.i.i.i.i.i.i unwind label %bb35.thread128.i.i.i.i.i.i, !noalias !ID

bb57.i.i.i.i.i.i:                                 ; preds = %bb7.i.i.i.i.i.i
  %_80.sroa.5.0._31.sroa_idx.i.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %_31.i.i.i.i.i.i, i64 56
  %val3.sroa.5.0._29.sroa_idx.i.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %_29.i.i.i.i.i.i, i64 56
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(296) %val3.sroa.5.0._29.sroa_idx.i.i.i.i.i.i, ptr noundef nonnull align 8 dereferenceable(296) %_80.sroa.5.0._31.sroa_idx.i.i.i.i.i.i, i64 296, i1 false), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_31.i.i.i.i.i.i), !noalias !ID
  %val3.sroa.4.0._29.sroa_idx.i.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %_29.i.i.i.i.i.i, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %val3.sroa.4.0._29.sroa_idx.i.i.i.i.i.i, ptr noundef nonnull align 8 dereferenceable(48) %_30.sroa.6.i.i.i.i.i.i, i64 48, i1 false), !noalias !ID
  store i64 %123, ptr %_29.i.i.i.i.i.i, align 8, !noalias !ID
  %127 = getelementptr inbounds nuw i8, ptr %_27.i.i.i.i.i.i, i64 88
  store ptr %_29.i.i.i.i.i.i, ptr %127, align 8, !noalias !ID
  %128 = getelementptr inbounds nuw i8, ptr %_11.i.i.i, i64 240
  %_37.sroa.0.0.copyload.i.i.i.i.i.i = load i64, ptr %128, align 8, !alias.scope !ID, !noalias !ID
  store i64 %_37.sroa.0.0.copyload.i.i.i.i.i.i, ptr %_27.i.i.i.i.i.i, align 8, !noalias !ID
; invoke <purrdf_sparql_eval::engine::NativeSparqlEngine>::query_governed
  invoke void @<purrdf_sparql_eval::engine::NativeSparqlEngine>::query_governed(ptr noalias nofree noundef nonnull sret([368 x i8]) align 8 captures(address) dereferenceable(368) %_22.i.i.i.i.i.i, ptr noundef nonnull align 8 %engine.i.i.i.i.i.i, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(8) %dataset.i.i.i.i.i.i, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(48) %_25.i.i.i.i.i.i, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(184) %_27.i.i.i.i.i.i, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(80) %governors.i.i.i)
          to label %bb8.i.i.i.i.i.i unwind label %cleanup10.i.i.i.i.i.i, !noalias !ID

cleanup10.i.i.i.i.i.i:                            ; preds = %bb1.i.i.i.i.i.i.i, %bb57.i.i.i.i.i.i
  %129 = landingpad { ptr, i32 }
          cleanup
  br label %cleanup10.body.i.i.i.i.i.i

cleanup10.body.i.i.i.i.i.i:                       ; preds = %cleanup.body.i48.i.i.i.i.i.i, %cleanup10.i.i.i.i.i.i
  %eh.lpad-body56.i.i.i.i.i.i = phi { ptr, i32 } [ %129, %cleanup10.i.i.i.i.i.i ], [ %eh.lpad-body.i49.i.i.i.i.i.i, %cleanup.body.i48.i.i.i.i.i.i ]
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::extension_env::ExtensionEnv>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::extension_env::ExtensionEnv>(ptr noalias nofree noundef align 8 dereferenceable(352) %_29.i.i.i.i.i.i) #ATTR
          to label %bb19.i.i.i.i.i.i unwind label %terminate.i.i.i.i.i.i, !noalias !ID

bb8.i.i.i.i.i.i:                                  ; preds = %bb57.i.i.i.i.i.i
  call void @llvm.lifetime.end.p0(ptr nonnull %_25.i.i.i.i.i.i), !noalias !ID
  %130 = load i64, ptr %_22.i.i.i.i.i.i, align 8, !range !ID, !noalias !ID, !noundef !ID
  %131 = icmp eq i64 %130, -2
  br i1 %131, label %bb59.i.i.i.i.i.i, label %bb60.i.i.i.i.i.i

bb59.i.i.i.i.i.i:                                 ; preds = %bb8.i.i.i.i.i.i
  %132 = getelementptr inbounds nuw i8, ptr %_22.i.i.i.i.i.i, i64 8
  call void @llvm.lifetime.start.p0(ptr nonnull %_90.i.i.i.i.i.i), !noalias !ID
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(96) %_90.i.i.i.i.i.i, ptr noundef nonnull align 8 dereferenceable(96) %132, i64 96, i1 false), !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %_89.i.i.i.i.i.i), !noalias !ID
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  call void @llvm.lifetime.start.p0(ptr nonnull %_3.i45.i.i.i.i.i.i), !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %args.i44.i.i.i.i.i.i), !noalias !ID
  store ptr %_90.i.i.i.i.i.i, ptr %args.i44.i.i.i.i.i.i, align 8, !noalias !ID
  %_7.sroa.4.0..sroa_idx.i46.i.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %args.i44.i.i.i.i.i.i, i64 8
  store ptr @<purrdf_core::diagnostic::RdfDiagnostic as core::fmt::Display>::fmt, ptr %_7.sroa.4.0..sroa_idx.i46.i.i.i.i.i.i, align 8, !noalias !ID
; invoke alloc::fmt::format::format_inner
  invoke void @alloc::fmt::format::format_inner(ptr noalias nofree noundef nonnull sret([24 x i8]) align 8 captures(none) dereferenceable(24) %_3.i45.i.i.i.i.i.i, ptr noundef nonnull @alloc_592fabd2ffa0e5a6b6713c88c8b00c99, ptr noundef nonnull %args.i44.i.i.i.i.i.i)
          to label %bb5.i52.i.i.i.i.i.i unwind label %cleanup.i47.i.i.i.i.i.i, !noalias !ID

cleanup.i47.i.i.i.i.i.i:                          ; preds = %bb4.i.i.i.i.i.i.i.i, %bb2.i.i54.i.i.i.i.i.i, %bb59.i.i.i.i.i.i
  %133 = landingpad { ptr, i32 }
          cleanup
  br label %cleanup.body.i48.i.i.i.i.i.i

cleanup.body.i48.i.i.i.i.i.i:                     ; preds = %bb2.i.i.i4.i.i.i.i53.i.i.i.i.i.i, %bb9.i.i.i.i.i.i.i.i, %cleanup.i47.i.i.i.i.i.i
  %eh.lpad-body.i49.i.i.i.i.i.i = phi { ptr, i32 } [ %133, %cleanup.i47.i.i.i.i.i.i ], [ %lpad.thr_comm.split-lp.i.i.i.i.i.i.i.i, %bb2.i.i.i4.i.i.i.i53.i.i.i.i.i.i ], [ %lpad.thr_comm.split-lp.i.i.i.i.i.i.i.i, %bb9.i.i.i.i.i.i.i.i ]
; invoke core::ptr::drop_glue::<purrdf_core::diagnostic::RdfDiagnostic>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_core::diagnostic::RdfDiagnostic>(ptr noalias nofree noundef nonnull align 8 dereferenceable(96) %_90.i.i.i.i.i.i) #ATTR
          to label %cleanup10.body.i.i.i.i.i.i unwind label %terminate.i50.i.i.i.i.i.i, !noalias !ID

bb5.i52.i.i.i.i.i.i:                              ; preds = %bb59.i.i.i.i.i.i
  call void @llvm.lifetime.end.p0(ptr nonnull %args.i44.i.i.i.i.i.i), !noalias !ID
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %134 = getelementptr inbounds nuw i8, ptr %_90.i.i.i.i.i.i, i64 80
  %135 = load ptr, ptr %134, align 8, !alias.scope !ID, !noalias !ID, !align !ID, !noundef !ID
  %136 = getelementptr inbounds nuw i8, ptr %_90.i.i.i.i.i.i, i64 8
  %_20.i.i.i.i.i.i.i.i = load ptr, ptr %136, align 8, !alias.scope !ID, !noalias !ID, !nonnull !ID, !noundef !ID
  %137 = getelementptr inbounds nuw i8, ptr %_90.i.i.i.i.i.i, i64 16
  %_19.i.i.i.i.i.i.i.i = load i64, ptr %137, align 8, !alias.scope !ID, !noalias !ID, !noundef !ID
; invoke purrdf_validate::xpath_regex::diagnostic_refusal_code
  %138 = invoke { ptr, i64 } @purrdf_validate::xpath_regex::diagnostic_refusal_code(ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) %_20.i.i.i.i.i.i.i.i, i64 noundef %_19.i.i.i.i.i.i.i.i)
          to label %bb1.i.i.i.i.i.i.i.i unwind label %bb9.i.i.i.i.i.i.i.i, !noalias !ID

bb1.i.i.i.i.i.i.i.i:                              ; preds = %bb5.i52.i.i.i.i.i.i
  %139 = extractvalue { ptr, i64 } %138, 0
  %.not3.i.i.i.i.i.i.i.i = icmp ne ptr %135, null
  %.not4.i.i.i.i.i.i.i.i = icmp eq ptr %139, null
  %or.cond.i.i.i.i.i.i.i.i = select i1 %.not3.i.i.i.i.i.i.i.i, i1 true, i1 %.not4.i.i.i.i.i.i.i.i
  br i1 %or.cond.i.i.i.i.i.i.i.i, label %bb2.i.i54.i.i.i.i.i.i, label %bb4.i.i.i.i.i.i.i.i

bb2.i.i54.i.i.i.i.i.i:                            ; preds = %bb1.i.i.i.i.i.i.i.i
; invoke purrdf_native::py_store::presentation::presented_value_error
  invoke fastcc void @purrdf_native::py_store::presentation::presented_value_error(ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(48) %_89.i.i.i.i.i.i, ptr noalias nofree noundef nonnull readonly align 8 captures(none) dereferenceable(24) %_3.i45.i.i.i.i.i.i, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable_or_null(80) %135)
          to label %bb1.i.i.i.i.i.i.i unwind label %cleanup.i47.i.i.i.i.i.i, !noalias !ID

bb4.i.i.i.i.i.i.i.i:                              ; preds = %bb1.i.i.i.i.i.i.i.i
  %140 = extractvalue { ptr, i64 } %138, 1
; invoke purrdf_native::py_store::presentation::refusal_value_error
  invoke fastcc void @purrdf_native::py_store::presentation::refusal_value_error(ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(48) %_89.i.i.i.i.i.i, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(24) %_3.i45.i.i.i.i.i.i, ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) %139, i64 noundef %140)
          to label %bb1.i.i.i.i.i.i.i unwind label %cleanup.i47.i.i.i.i.i.i, !noalias !ID

bb9.i.i.i.i.i.i.i.i:                              ; preds = %bb5.i52.i.i.i.i.i.i
  %lpad.thr_comm.split-lp.i.i.i.i.i.i.i.i = landingpad { ptr, i32 }
          cleanup
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %_1.val.i.i.i.i.i.i.i.i.i = load i64, ptr %_3.i45.i.i.i.i.i.i, align 8, !alias.scope !ID, !noalias !ID
  %141 = icmp eq i64 %_1.val.i.i.i.i.i.i.i.i.i, 0
  br i1 %141, label %cleanup.body.i48.i.i.i.i.i.i, label %bb2.i.i.i4.i.i.i.i53.i.i.i.i.i.i

bb2.i.i.i4.i.i.i.i53.i.i.i.i.i.i:                 ; preds = %bb9.i.i.i.i.i.i.i.i
  %142 = getelementptr inbounds nuw i8, ptr %_3.i45.i.i.i.i.i.i, i64 8
  %_1.val1.i.i.i.i.i.i.i.i.i = load ptr, ptr %142, align 8, !alias.scope !ID, !noalias !ID, !nonnull !ID, !noundef !ID
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %_1.val1.i.i.i.i.i.i.i.i.i, i64 noundef %_1.val.i.i.i.i.i.i.i.i.i, i64 noundef range(i64 1, -9223372036854775807) 1) #ATTR, !noalias !ID
  br label %cleanup.body.i48.i.i.i.i.i.i

bb1.i.i.i.i.i.i.i:                                ; preds = %bb4.i.i.i.i.i.i.i.i, %bb2.i.i54.i.i.i.i.i.i
; invoke core::ptr::drop_glue::<purrdf_core::diagnostic::RdfDiagnostic>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_core::diagnostic::RdfDiagnostic>(ptr noalias nofree noundef nonnull align 8 dereferenceable(96) %_90.i.i.i.i.i.i)
          to label %bb61.i.i.i.i.i.i unwind label %cleanup10.i.i.i.i.i.i, !noalias !ID

terminate.i50.i.i.i.i.i.i:                        ; preds = %cleanup.body.i48.i.i.i.i.i.i
  %143 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #ATTR, !noalias !ID
  unreachable

bb60.i.i.i.i.i.i:                                 ; preds = %bb8.i.i.i.i.i.i
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(368) %outcome.i.i.i, ptr noundef nonnull align 8 dereferenceable(368) %_22.i.i.i.i.i.i, i64 368, i1 false), !noalias !ID
  br label %bb58.i.i.i.i.i.i

bb58.i.i.i.i.i.i:                                 ; preds = %bb61.i.i.i.i.i.i, %bb60.i.i.i.i.i.i
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::extension_env::ExtensionEnv>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::extension_env::ExtensionEnv>(ptr noalias nofree noundef align 8 dereferenceable(352) %_29.i.i.i.i.i.i)
          to label %bb9.i.i.i.i.i.i unwind label %cleanup9.i.i.i.i.i.i, !noalias !ID

bb61.i.i.i.i.i.i:                                 ; preds = %bb1.i.i.i.i.i.i.i
  call void @llvm.lifetime.end.p0(ptr nonnull %_3.i45.i.i.i.i.i.i), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_90.i.i.i.i.i.i), !noalias !ID
  %144 = getelementptr inbounds nuw i8, ptr %outcome.i.i.i, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %144, ptr noundef nonnull align 8 dereferenceable(48) %_89.i.i.i.i.i.i, i64 48, i1 false), !noalias !ID
  store i64 -2, ptr %outcome.i.i.i, align 8, !alias.scope !ID, !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_89.i.i.i.i.i.i), !noalias !ID
  br label %bb58.i.i.i.i.i.i

bb9.i.i.i.i.i.i:                                  ; preds = %bb58.i.i.i.i.i.i
  call void @llvm.lifetime.end.p0(ptr nonnull %_30.sroa.6.i.i.i.i.i.i)
  call void @llvm.lifetime.end.p0(ptr nonnull %_29.i.i.i.i.i.i), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_22.i.i.i.i.i.i), !noalias !ID
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::engine::NativeSparqlEngine>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::engine::NativeSparqlEngine>(ptr noalias nofree noundef align 8 dereferenceable(520) %engine.i.i.i.i.i.i)
          to label %bb10.i.i.i.i.i.i unwind label %bb35.thread128.i.i.i.i.i.i, !noalias !ID

bb10.i.i.i.i.i.i:                                 ; preds = %bb9.i.i.i.i.i.i
  call void @llvm.lifetime.end.p0(ptr nonnull %engine.i.i.i.i.i.i), !noalias !ID
  br i1 %.not23.i.i.i.i.i.i, label %bb11.i.i.i.i.i.i, label %bb2.i57.i.i.i.i.i.i

bb2.i57.i.i.i.i.i.i:                              ; preds = %bb10.i.i.i.i.i.i
; invoke core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, alloc::sync::Arc<dyn purrdf_sparql_eval::agg_fn::CustomAggregate>)>>
  invoke fastcc void @core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, alloc::sync::Arc<dyn purrdf_sparql_eval::agg_fn::CustomAggregate>)>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(40) %aggregates.i.i.i.i.i.i)
          to label %bb11.i.i.i.i.i.i unwind label %cleanup6.i.i.i.i.i.i, !noalias !ID

bb11.i.i.i.i.i.i:                                 ; preds = %bb2.i57.i.i.i.i.i.i, %bb10.i.i.i.i.i.i
  call void @llvm.lifetime.end.p0(ptr nonnull %aggregates.i.i.i.i.i.i), !noalias !ID
  br i1 %.not22.i.i.i.i.i.i, label %bb12.i.i.i.i.i.i, label %bb2.i61.i.i.i.i.i.i

bb2.i61.i.i.i.i.i.i:                              ; preds = %bb11.i.i.i.i.i.i
; invoke core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, alloc::sync::Arc<dyn purrdf_sparql_eval::property_fn::PropertyFunction>)>>
  invoke fastcc void @core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, alloc::sync::Arc<dyn purrdf_sparql_eval::property_fn::PropertyFunction>)>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(72) %registry.i.i.i.i.i.i)
          to label %core::ptr::drop_glue::<purrdf_sparql_eval::property_fn::PropertyFunctionRegistry> (.exit.i.i.i.i.i.i.i) unwind label %cleanup.i.i62.i.i.i.i.i.i, !noalias !ID

cleanup.i.i62.i.i.i.i.i.i:                        ; preds = %bb2.i61.i.i.i.i.i.i
  %145 = landingpad { ptr, i32 }
          cleanup
  %146 = getelementptr inbounds nuw i8, ptr %registry.i.i.i.i.i.i, i64 32
; invoke core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, purrdf_sparql_eval::property_fn::RankedDeclaration)>>
  invoke fastcc void @core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, purrdf_sparql_eval::property_fn::RankedDeclaration)>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(32) %146)
          to label %bb22.i.i.i.i.i.i unwind label %terminate.i.i.i.i.i.i.i.i, !noalias !ID

terminate.i.i.i.i.i.i.i.i:                        ; preds = %cleanup.i.i62.i.i.i.i.i.i
  %147 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #ATTR, !noalias !ID
  unreachable

core::ptr::drop_glue::<purrdf_sparql_eval::property_fn::PropertyFunctionRegistry> (.exit.i.i.i.i.i.i.i): ; preds = %bb2.i61.i.i.i.i.i.i
  %148 = getelementptr inbounds nuw i8, ptr %registry.i.i.i.i.i.i, i64 32
; invoke core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, purrdf_sparql_eval::property_fn::RankedDeclaration)>>
  invoke fastcc void @core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, purrdf_sparql_eval::property_fn::RankedDeclaration)>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(32) %148)
          to label %bb12.i.i.i.i.i.i unwind label %cleanup5.i.i.i.i.i.i, !noalias !ID

bb12.i.i.i.i.i.i:                                 ; preds = %core::ptr::drop_glue::<purrdf_sparql_eval::property_fn::PropertyFunctionRegistry> (.exit.i.i.i.i.i.i.i), %bb11.i.i.i.i.i.i
  call void @llvm.lifetime.end.p0(ptr nonnull %registry.i.i.i.i.i.i), !noalias !ID
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %_10.i.i67.i.i.i.i.i.i = load ptr, ptr %dataset.i.i.i.i.i.i, align 8, !alias.scope !ID, !noalias !ID, !nonnull !ID, !noundef !ID
  %_2.i.i68.i.i.i.i.i.i = atomicrmw sub ptr %_10.i.i67.i.i.i.i.i.i, i64 1 release, align 8, !noalias !ID
  %149 = icmp eq i64 %_2.i.i68.i.i.i.i.i.i, 1
  br i1 %149, label %bb2.i.i69.i.i.i.i.i.i, label %bb13.i.i.i.i.i.i

bb2.i.i69.i.i.i.i.i.i:                            ; preds = %bb12.i.i.i.i.i.i
  fence acquire
; invoke <alloc::sync::Arc<purrdf_core::ir::dataset::RdfDataset>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_core::ir::dataset::RdfDataset>>::drop_slow(ptr noalias nofree noundef nonnull align 8 dereferenceable(8) %dataset.i.i.i.i.i.i) #ATTR
          to label %bb13.i.i.i.i.i.i unwind label %bb41.i.i.i.i.i.i, !noalias !ID

bb13.i.i.i.i.i.i:                                 ; preds = %bb2.i.i69.i.i.i.i.i.i, %bb12.i.i.i.i.i.i
  call void @llvm.lifetime.end.p0(ptr nonnull %dataset.i.i.i.i.i.i), !noalias !ID
  %150 = icmp sgt i64 %93, 0
  br i1 %150, label %bb2.i.i.i4.i.i.i.i.i.i.i.i.i, label %<purrdf_native::py_store::quad_store::PyQuadStore>::query_governed::{closure#0}::{closure#0} (.exit.i.i.i.i.i)

bb2.i.i.i4.i.i.i.i.i.i.i.i.i:                     ; preds = %bb13.i.i.i.i.i.i
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %_67.i.i.i.i.i.i, i64 noundef %93, i64 noundef range(i64 1, -9223372036854775807) 1) #ATTR, !noalias !ID
  br label %<purrdf_native::py_store::quad_store::PyQuadStore>::query_governed::{closure#0}::{closure#0} (.exit.i.i.i.i.i)

terminate.i.i.i.i.i.i:                            ; preds = %bb36.i.i.i.i.i.i, %bb40.i.i.i.i.i.i, %bb28.i.i.i.i.i.i, %cleanup10.body.i.i.i.i.i.i, %bb19.i.i.i.i.i.i, %bb2.i.i.i.i.i.i.i, %bb21.i.i.i.i.i.i, %bb2.i.i.i.i.i.i.i.i
  %151 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #ATTR, !noalias !ID
  unreachable

bb14.i.i.i.i.i.i:                                 ; preds = %bb56.i.i.i.i.i.i
  call void @llvm.lifetime.end.p0(ptr nonnull %engine.i.i.i.i.i.i), !noalias !ID
  br i1 %.not23.i.i.i.i.i.i, label %bb15.i.i.i.i.i.i, label %bb2.i74.i.i.i.i.i.i

bb2.i74.i.i.i.i.i.i:                              ; preds = %bb14.i.i.i.i.i.i
; invoke core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, alloc::sync::Arc<dyn purrdf_sparql_eval::agg_fn::CustomAggregate>)>>
  invoke fastcc void @core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, alloc::sync::Arc<dyn purrdf_sparql_eval::agg_fn::CustomAggregate>)>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(40) %aggregates.i.i.i.i.i.i)
          to label %bb15.i.i.i.i.i.i unwind label %cleanup6.i.i.i.i.i.i, !noalias !ID

bb15.i.i.i.i.i.i:                                 ; preds = %bb2.i74.i.i.i.i.i.i, %bb14.i.i.i.i.i.i
  call void @llvm.lifetime.end.p0(ptr nonnull %aggregates.i.i.i.i.i.i), !noalias !ID
  br i1 %.not22.i.i.i.i.i.i, label %bb16.i.i.i.i.i.i, label %bb2.i78.i.i.i.i.i.i

bb2.i78.i.i.i.i.i.i:                              ; preds = %bb15.i.i.i.i.i.i
; invoke core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, alloc::sync::Arc<dyn purrdf_sparql_eval::property_fn::PropertyFunction>)>>
  invoke fastcc void @core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, alloc::sync::Arc<dyn purrdf_sparql_eval::property_fn::PropertyFunction>)>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(72) %registry.i.i.i.i.i.i)
          to label %core::ptr::drop_glue::<purrdf_sparql_eval::property_fn::PropertyFunctionRegistry> (.exit.i82.i.i.i.i.i.i) unwind label %cleanup.i.i79.i.i.i.i.i.i, !noalias !ID

cleanup.i.i79.i.i.i.i.i.i:                        ; preds = %bb2.i78.i.i.i.i.i.i
  %152 = landingpad { ptr, i32 }
          cleanup
  %153 = getelementptr inbounds nuw i8, ptr %registry.i.i.i.i.i.i, i64 32
; invoke core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, purrdf_sparql_eval::property_fn::RankedDeclaration)>>
  invoke fastcc void @core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, purrdf_sparql_eval::property_fn::RankedDeclaration)>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(32) %153)
          to label %bb22.i.i.i.i.i.i unwind label %terminate.i.i80.i.i.i.i.i.i, !noalias !ID

terminate.i.i80.i.i.i.i.i.i:                      ; preds = %cleanup.i.i79.i.i.i.i.i.i
  %154 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #ATTR, !noalias !ID
  unreachable

core::ptr::drop_glue::<purrdf_sparql_eval::property_fn::PropertyFunctionRegistry> (.exit.i82.i.i.i.i.i.i): ; preds = %bb2.i78.i.i.i.i.i.i
  %155 = getelementptr inbounds nuw i8, ptr %registry.i.i.i.i.i.i, i64 32
; invoke core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, purrdf_sparql_eval::property_fn::RankedDeclaration)>>
  invoke fastcc void @core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, purrdf_sparql_eval::property_fn::RankedDeclaration)>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(32) %155)
          to label %bb16.i.i.i.i.i.i unwind label %cleanup5.i.i.i.i.i.i, !noalias !ID

bb16.i.i.i.i.i.i:                                 ; preds = %core::ptr::drop_glue::<purrdf_sparql_eval::property_fn::PropertyFunctionRegistry> (.exit.i82.i.i.i.i.i.i), %bb15.i.i.i.i.i.i, %bb45.i.i.i.i.i.i
  %_40.sroa.0.7.i.i.i.i.i.i = phi i8 [ 1, %bb45.i.i.i.i.i.i ], [ 0, %core::ptr::drop_glue::<purrdf_sparql_eval::property_fn::PropertyFunctionRegistry> (.exit.i82.i.i.i.i.i.i) ], [ 0, %bb15.i.i.i.i.i.i ]
  call void @llvm.lifetime.end.p0(ptr nonnull %registry.i.i.i.i.i.i), !noalias !ID
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %_10.i.i89.i.i.i.i.i.i = load ptr, ptr %dataset.i.i.i.i.i.i, align 8, !alias.scope !ID, !noalias !ID, !nonnull !ID, !noundef !ID
  %_2.i.i90.i.i.i.i.i.i = atomicrmw sub ptr %_10.i.i89.i.i.i.i.i.i, i64 1 release, align 8, !noalias !ID
  %156 = icmp eq i64 %_2.i.i90.i.i.i.i.i.i, 1
  br i1 %156, label %bb2.i.i91.i.i.i.i.i.i, label %bb62.i.i.i.i.i.i

bb2.i.i91.i.i.i.i.i.i:                            ; preds = %bb16.i.i.i.i.i.i
  fence acquire
; invoke <alloc::sync::Arc<purrdf_core::ir::dataset::RdfDataset>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_core::ir::dataset::RdfDataset>>::drop_slow(ptr noalias nofree noundef nonnull align 8 dereferenceable(8) %dataset.i.i.i.i.i.i) #ATTR
          to label %bb62.i.i.i.i.i.i unwind label %bb41.i.i.i.i.i.i, !noalias !ID

bb34.i.i.i.i.i.i:                                 ; preds = %bb5.i.i.i.i.i.i
  %lpad.thr_comm.split-lp127.i.i.i.i.i.i = landingpad { ptr, i32 }
          cleanup
; call core::ptr::drop_glue::<purrdf_sparql_algebra::parser::ParserOptions>
  call fastcc void @core::ptr::drop_glue::<purrdf_sparql_algebra::parser::ParserOptions>(ptr noalias nofree noundef align 8 dereferenceable(72) %parser_options.i.i.i.i.i.i) #ATTR, !noalias !ID
  br label %bb20.i.i.i.i.i.i

bb62.i.i.i.i.i.i:                                 ; preds = %bb2.i.i91.i.i.i.i.i.i, %bb16.i.i.i.i.i.i
  call void @llvm.lifetime.end.p0(ptr nonnull %dataset.i.i.i.i.i.i), !noalias !ID
  %157 = trunc nuw i8 %_40.sroa.0.7.i.i.i.i.i.i to i1
  br label %bb33.i.i.i.i.i.i

bb33.i.i.i.i.i.i:                                 ; preds = %bb2.i.i.i6.i.i.i.i.i.i, %bb4.i4.i.i.i.i.i, %bb62.i.i.i.i.i.i
  %_40.sroa.0.8.i.i.i.i.i.i = phi i1 [ %157, %bb62.i.i.i.i.i.i ], [ true, %bb4.i4.i.i.i.i.i ], [ true, %bb2.i.i.i6.i.i.i.i.i.i ]
  %158 = getelementptr inbounds nuw i8, ptr %_11.i.i.i, i64 48
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %159 = load i64, ptr %158, align 8, !range !ID, !alias.scope !ID, !noalias !ID, !noundef !ID
  %160 = icmp eq i64 %159, -1
  br i1 %160, label %bb31.i.i.i.i.i.i, label %bb2.i94.i.i.i.i.i.i

bb2.i94.i.i.i.i.i.i:                              ; preds = %bb33.i.i.i.i.i.i
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %161 = icmp eq i64 %159, 0
  br i1 %161, label %bb31.i.i.i.i.i.i, label %bb2.i.i.i4.i.i.i95.i.i.i.i.i.i

bb2.i.i.i4.i.i.i95.i.i.i.i.i.i:                   ; preds = %bb2.i94.i.i.i.i.i.i
  %162 = getelementptr inbounds nuw i8, ptr %_11.i.i.i, i64 56
  %_1.val1.i.i96.i.i.i.i.i.i = load ptr, ptr %162, align 8, !alias.scope !ID, !noalias !ID, !nonnull !ID, !noundef !ID
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %_1.val1.i.i96.i.i.i.i.i.i, i64 noundef %159, i64 noundef range(i64 1, -9223372036854775807) 1) #ATTR, !noalias !ID
  br label %bb31.i.i.i.i.i.i

bb44.i.i.i.i.i.i:                                 ; preds = %bb8.i.i.i.i.i.i.i
  call void @llvm.lifetime.end.p0(ptr nonnull %_3.i.i.i.i.i.i.i), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_47.i.i.i.i.i.i), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_6.i.i.i.i.i.i), !noalias !ID
  %163 = getelementptr inbounds nuw i8, ptr %outcome.i.i.i, i64 8
  %_54.sroa.4.sroa.4.0._54.sroa.4.0..sroa_idx.sroa_idx.i.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %outcome.i.i.i, i64 24
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %163, i8 0, i64 16, i1 false), !alias.scope !ID, !noalias !ID
  store i64 1, ptr %_54.sroa.4.sroa.4.0._54.sroa.4.0..sroa_idx.sroa_idx.i.i.i.i.i.i, align 8, !alias.scope !ID, !noalias !ID
  %_54.sroa.4.sroa.5.0._54.sroa.4.0..sroa_idx.sroa_idx.i.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %outcome.i.i.i, i64 32
  store ptr %81, ptr %_54.sroa.4.sroa.5.0._54.sroa.4.0..sroa_idx.sroa_idx.i.i.i.i.i.i, align 8, !alias.scope !ID, !noalias !ID
  %_54.sroa.4.sroa.6.0._54.sroa.4.0..sroa_idx.sroa_idx.i.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %outcome.i.i.i, i64 40
  store ptr @vtable.22, ptr %_54.sroa.4.sroa.6.0._54.sroa.4.0..sroa_idx.sroa_idx.i.i.i.i.i.i, align 8, !alias.scope !ID, !noalias !ID
  %_54.sroa.4.sroa.7.0._54.sroa.4.0..sroa_idx.sroa_idx.i.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %outcome.i.i.i, i64 48
  store i32 3, ptr %_54.sroa.4.sroa.7.0._54.sroa.4.0..sroa_idx.sroa_idx.i.i.i.i.i.i, align 8, !alias.scope !ID, !noalias !ID
  store i64 -2, ptr %outcome.i.i.i, align 8, !alias.scope !ID, !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %dataset.i.i.i.i.i.i), !noalias !ID
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %164 = getelementptr inbounds nuw i8, ptr %_11.i.i.i, i64 8
  %_1.val.i.i.i.i.i.i = load ptr, ptr %164, align 8, !alias.scope !ID, !noalias !ID, !nonnull !ID, !noundef !ID
  %165 = getelementptr inbounds nuw i8, ptr %_11.i.i.i, i64 16
  %_1.val1.i.i.i.i.i.i = load i64, ptr %165, align 8, !alias.scope !ID, !noalias !ID, !noundef !ID
  %_7.i.i.i.i.i.i.i.i28 = icmp eq i64 %_1.val1.i.i.i.i.i.i, 0
  br i1 %_7.i.i.i.i.i.i.i.i28, label %bb4.i4.i.i.i.i.i, label %bb5.i.i.i.i.i.i.i.i

bb6.i.i.i.i.i.i.i.i:                              ; preds = %bb5.i.i.i.i.i.i.i.i
  %_7.i.i.i.i.i.i.i.i = icmp eq i64 %166, %_1.val1.i.i.i.i.i.i
  br i1 %_7.i.i.i.i.i.i.i.i, label %bb4.i4.i.i.i.i.i, label %bb5.i.i.i.i.i.i.i.i

bb5.i.i.i.i.i.i.i.i:                              ; preds = %bb44.i.i.i.i.i.i, %bb6.i.i.i.i.i.i.i.i
  %_3.sroa.0.0.i.i.i.i.i.i.i.i29 = phi i64 [ %166, %bb6.i.i.i.i.i.i.i.i ], [ 0, %bb44.i.i.i.i.i.i ]
  %_6.i.i.i.i.i.i.i.i = getelementptr inbounds nuw [160 x i8], ptr %_1.val.i.i.i.i.i.i, i64 %_3.sroa.0.0.i.i.i.i.i.i.i.i29
  %166 = add nuw nsw i64 %_3.sroa.0.0.i.i.i.i.i.i.i.i29, 1
; invoke core::ptr::drop_glue::<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>
  invoke fastcc void @core::ptr::drop_glue::<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>(ptr noalias nofree noundef align 8 dereferenceable(160) %_6.i.i.i.i.i.i.i.i)
          to label %bb6.i.i.i.i.i.i.i.i unwind label %cleanup.i.i.i1.i.i.i.i.i, !noalias !ID

bb4.i.i.i2.i.i.i.i.i:                             ; preds = %bb3.i.i.i.i.i.i.i.i
  %167 = add i64 %_3.sroa.0.1.i.i.i.i.i.i.i.i31, 1
  %_5.i.i.i.i.i.i.i.i = icmp eq i64 %167, %_1.val1.i.i.i.i.i.i
  br i1 %_5.i.i.i.i.i.i.i.i, label %cleanup.body.i.i.i.i.i.i, label %bb3.i.i.i.i.i.i.i.i

cleanup.i.i.i1.i.i.i.i.i:                         ; preds = %bb5.i.i.i.i.i.i.i.i
  %168 = landingpad { ptr, i32 }
          cleanup
  %_5.i.i.i.i.i.i.i.i30 = icmp eq i64 %166, %_1.val1.i.i.i.i.i.i
  br i1 %_5.i.i.i.i.i.i.i.i30, label %cleanup.body.i.i.i.i.i.i, label %bb3.i.i.i.i.i.i.i.i

bb3.i.i.i.i.i.i.i.i:                              ; preds = %cleanup.i.i.i1.i.i.i.i.i, %bb4.i.i.i2.i.i.i.i.i
  %_3.sroa.0.1.i.i.i.i.i.i.i.i31 = phi i64 [ %167, %bb4.i.i.i2.i.i.i.i.i ], [ %166, %cleanup.i.i.i1.i.i.i.i.i ]
  %_4.i.i.i.i.i.i.i.i = getelementptr inbounds nuw [160 x i8], ptr %_1.val.i.i.i.i.i.i, i64 %_3.sroa.0.1.i.i.i.i.i.i.i.i31
; invoke core::ptr::drop_glue::<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>
  invoke fastcc void @core::ptr::drop_glue::<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>(ptr noalias nofree noundef align 8 dereferenceable(160) %_4.i.i.i.i.i.i.i.i) #ATTR
          to label %bb4.i.i.i2.i.i.i.i.i unwind label %terminate.i.i.i3.i.i.i.i.i, !noalias !ID

terminate.i.i.i3.i.i.i.i.i:                       ; preds = %bb3.i.i.i.i.i.i.i.i
  %169 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #ATTR, !noalias !ID
  unreachable

cleanup.body.i.i.i.i.i.i:                         ; preds = %bb4.i.i.i2.i.i.i.i.i, %cleanup.i.i.i1.i.i.i.i.i
  %_1.val2.i.i.i.i.i.i = load i64, ptr %_11.i.i.i, align 8, !alias.scope !ID, !noalias !ID
  %170 = icmp eq i64 %_1.val2.i.i.i.i.i.i, 0
  br i1 %170, label %cleanup12.i.body.i.i.i.i.i, label %bb2.i.i.i.i.i.i.i.i.i

bb2.i.i.i.i.i.i.i.i.i:                            ; preds = %cleanup.body.i.i.i.i.i.i
  %alloc_size.i.i.i.i.i.i.i.i.i.i = mul nuw i64 %_1.val2.i.i.i.i.i.i, 160
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %_1.val.i.i.i.i.i.i, i64 noundef %alloc_size.i.i.i.i.i.i.i.i.i.i, i64 noundef range(i64 1, -9223372036854775807) 8) #ATTR, !noalias !ID
  br label %cleanup12.i.body.i.i.i.i.i

bb4.i4.i.i.i.i.i:                                 ; preds = %bb6.i.i.i.i.i.i.i.i, %bb44.i.i.i.i.i.i
  %_1.val4.i.i.i.i.i.i = load i64, ptr %_11.i.i.i, align 8, !alias.scope !ID, !noalias !ID
  %171 = icmp eq i64 %_1.val4.i.i.i.i.i.i, 0
  br i1 %171, label %bb33.i.i.i.i.i.i, label %bb2.i.i.i6.i.i.i.i.i.i

bb2.i.i.i6.i.i.i.i.i.i:                           ; preds = %bb4.i4.i.i.i.i.i
  %alloc_size.i.i.i.i7.i.i.i.i.i.i = mul nuw i64 %_1.val4.i.i.i.i.i.i, 160
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %_1.val.i.i.i.i.i.i, i64 noundef %alloc_size.i.i.i.i7.i.i.i.i.i.i, i64 noundef range(i64 1, -9223372036854775807) 8) #ATTR, !noalias !ID
  br label %bb33.i.i.i.i.i.i

cleanup12.i.body.i.i.i.i.i:                       ; preds = %bb2.i.i.i.i.i.i.i.i.i, %cleanup.body.i.i.i.i.i.i
  %172 = getelementptr inbounds nuw i8, ptr %_11.i.i.i, i64 48
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %173 = load i64, ptr %172, align 8, !range !ID, !alias.scope !ID, !noalias !ID, !noundef !ID
  %174 = icmp eq i64 %173, -1
  br i1 %174, label %bb28.i.i.i.i.i.i, label %bb2.i99.i.i.i.i.i.i

bb2.i99.i.i.i.i.i.i:                              ; preds = %cleanup12.i.body.i.i.i.i.i
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %175 = icmp eq i64 %173, 0
  br i1 %175, label %bb28.i.i.i.i.i.i, label %bb2.i.i.i4.i.i.i100.i.i.i.i.i.i

bb2.i.i.i4.i.i.i100.i.i.i.i.i.i:                  ; preds = %bb2.i99.i.i.i.i.i.i
  %176 = getelementptr inbounds nuw i8, ptr %_11.i.i.i, i64 56
  %_1.val1.i.i101.i.i.i.i.i.i = load ptr, ptr %176, align 8, !alias.scope !ID, !noalias !ID, !nonnull !ID, !noundef !ID
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %_1.val1.i.i101.i.i.i.i.i.i, i64 noundef %173, i64 noundef range(i64 1, -9223372036854775807) 1) #ATTR, !noalias !ID
  br label %bb28.i.i.i.i.i.i

bb31.i.i.i.i.i.i:                                 ; preds = %bb2.i.i.i4.i.i.i95.i.i.i.i.i.i, %bb2.i94.i.i.i.i.i.i, %bb33.i.i.i.i.i.i
  br i1 %_40.sroa.0.8.i.i.i.i.i.i, label %bb32.i.i.i.i.i.i, label %<purrdf_native::py_store::quad_store::PyQuadStore>::query_governed::{closure#0}::{closure#0} (.exit.i.i.i.i.i)

bb32.i.i.i.i.i.i:                                 ; preds = %bb31.i.i.i.i.i.i
  %177 = getelementptr inbounds nuw i8, ptr %_11.i.i.i, i64 72
; call core::ptr::drop_glue::<purrdf_native::py_store::query::EngineConfig>
  call fastcc void @core::ptr::drop_glue::<purrdf_native::py_store::query::EngineConfig>(ptr noalias nofree noundef readonly align 8 dereferenceable(168) %177), !noalias !ID
  br label %<purrdf_native::py_store::quad_store::PyQuadStore>::query_governed::{closure#0}::{closure#0} (.exit.i.i.i.i.i)

bb28.i.i.i.i.i.i:                                 ; preds = %bb2.i.i.i4.i.i.i100.i.i.i.i.i.i, %bb2.i99.i.i.i.i.i.i, %cleanup12.i.body.i.i.i.i.i
  %178 = getelementptr inbounds nuw i8, ptr %_11.i.i.i, i64 72
; call core::ptr::drop_glue::<purrdf_native::py_store::query::EngineConfig>
  call fastcc void @core::ptr::drop_glue::<purrdf_native::py_store::query::EngineConfig>(ptr noalias nofree noundef readonly align 8 dereferenceable(168) %178) #ATTR, !noalias !ID
  %179 = getelementptr inbounds nuw i8, ptr %_11.i.i.i, i64 24
; invoke core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_core::ir::term::TermValue)>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_core::ir::term::TermValue)>>(ptr noalias nofree noundef readonly align 8 dereferenceable(24) %179) #ATTR
          to label %cleanup1.body.i.i.i.i unwind label %terminate.i.i.i.i.i.i, !noalias !ID

bb39.i.i.i.i.i.i:                                 ; preds = %bb40.i.i.i.i.i.i, %bb2.i.i.i.i.i.i.i.i, %bb22.i.i.i.i.i.i, %bb41.i.i.i.i.i.i
  %.pn32113.i.i.i.i.i.i = phi { ptr, i32 } [ %lpad.thr_comm.split-lp.i.i.i.i.i.i, %bb41.i.i.i.i.i.i ], [ %eh.lpad-body119.i.i.i.i.i.i, %bb40.i.i.i.i.i.i ], [ %.pn30.i.i.i.i.i.i, %bb22.i.i.i.i.i.i ], [ %.pn30.i.i.i.i.i.i, %bb2.i.i.i.i.i.i.i.i ]
  %_40.sroa.0.0112.i.i.i.i.i.i = phi i8 [ %_40.sroa.0.1.ph.i.i.i.i.i.i, %bb41.i.i.i.i.i.i ], [ 1, %bb40.i.i.i.i.i.i ], [ %_40.sroa.0.2.i.i.i.i.i.i, %bb22.i.i.i.i.i.i ], [ %_40.sroa.0.2.i.i.i.i.i.i, %bb2.i.i.i.i.i.i.i.i ]
  %180 = getelementptr inbounds nuw i8, ptr %_11.i.i.i, i64 48
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %181 = load i64, ptr %180, align 8, !range !ID, !alias.scope !ID, !noalias !ID, !noundef !ID
  %182 = icmp eq i64 %181, -1
  br i1 %182, label %bb37.i.i.i.i.i.i, label %bb2.i104.i.i.i.i.i.i

bb2.i104.i.i.i.i.i.i:                             ; preds = %bb39.i.i.i.i.i.i
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %183 = icmp eq i64 %181, 0
  br i1 %183, label %bb37.i.i.i.i.i.i, label %bb2.i.i.i4.i.i.i105.i.i.i.i.i.i

bb2.i.i.i4.i.i.i105.i.i.i.i.i.i:                  ; preds = %bb2.i104.i.i.i.i.i.i
  %184 = getelementptr inbounds nuw i8, ptr %_11.i.i.i, i64 56
  %_1.val1.i.i106.i.i.i.i.i.i = load ptr, ptr %184, align 8, !alias.scope !ID, !noalias !ID, !nonnull !ID, !noundef !ID
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %_1.val1.i.i106.i.i.i.i.i.i, i64 noundef %181, i64 noundef range(i64 1, -9223372036854775807) 1) #ATTR, !noalias !ID
  br label %bb37.i.i.i.i.i.i

bb40.i.i.i.i.i.i:                                 ; preds = %cleanup.body.i.i.i.i.i.i.i, %bb41.thread120.i.i.i.i.i.i
  %eh.lpad-body119.i.i.i.i.i.i = phi { ptr, i32 } [ %lpad.thr_comm.i.i.i.i.i.i, %bb41.thread120.i.i.i.i.i.i ], [ %eh.lpad-body.i.i.i.i.i.i.i, %cleanup.body.i.i.i.i.i.i.i ]
; invoke core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(280) %_11.i.i.i) #ATTR
          to label %bb39.i.i.i.i.i.i unwind label %terminate.i.i.i.i.i.i, !noalias !ID

bb37.i.i.i.i.i.i:                                 ; preds = %bb2.i.i.i4.i.i.i105.i.i.i.i.i.i, %bb2.i104.i.i.i.i.i.i, %bb39.i.i.i.i.i.i
  %185 = trunc nuw i8 %_40.sroa.0.0112.i.i.i.i.i.i to i1
  br i1 %185, label %bb38.i.i.i.i.i.i, label %bb36.i.i.i.i.i.i

bb36.i.i.i.i.i.i:                                 ; preds = %bb38.i.i.i.i.i.i, %bb37.i.i.i.i.i.i
  %186 = getelementptr inbounds nuw i8, ptr %_11.i.i.i, i64 24
; invoke core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_core::ir::term::TermValue)>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_core::ir::term::TermValue)>>(ptr noalias nofree noundef readonly align 8 dereferenceable(24) %186) #ATTR
          to label %cleanup1.body.i.i.i.i unwind label %terminate.i.i.i.i.i.i, !noalias !ID

bb38.i.i.i.i.i.i:                                 ; preds = %bb37.i.i.i.i.i.i
  %187 = getelementptr inbounds nuw i8, ptr %_11.i.i.i, i64 72
; call core::ptr::drop_glue::<purrdf_native::py_store::query::EngineConfig>
  call fastcc void @core::ptr::drop_glue::<purrdf_native::py_store::query::EngineConfig>(ptr noalias nofree noundef readonly align 8 dereferenceable(168) %187) #ATTR, !noalias !ID
  br label %bb36.i.i.i.i.i.i

<purrdf_native::py_store::quad_store::PyQuadStore>::query_governed::{closure#0}::{closure#0} (.exit.i.i.i.i.i): ; preds = %bb32.i.i.i.i.i.i, %bb31.i.i.i.i.i.i, %bb2.i.i.i4.i.i.i.i.i.i.i.i.i, %bb13.i.i.i.i.i.i
  %.sink.i.i.i.i.i.i = getelementptr inbounds nuw i8, ptr %_11.i.i.i, i64 24
; invoke core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_core::ir::term::TermValue)>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_core::ir::term::TermValue)>>(ptr noalias nofree noundef readonly align 8 dereferenceable(24) %.sink.i.i.i.i.i.i)
          to label %bb2.i.i.i.i unwind label %cleanup1.i.i.i.i, !noalias !ID

cleanup1.i.i.i.i:                                 ; preds = %<purrdf_native::py_store::quad_store::PyQuadStore>::query_governed::{closure#0}::{closure#0} (.exit.i.i.i.i.i)
  %188 = landingpad { ptr, i32 }
          cleanup
  br label %cleanup1.body.i.i.i.i

cleanup1.body.i.i.i.i:                            ; preds = %cleanup1.i.i.i.i, %bb36.i.i.i.i.i.i, %bb28.i.i.i.i.i.i
  %eh.lpad-body.i.i.i.i = phi { ptr, i32 } [ %188, %cleanup1.i.i.i.i ], [ %168, %bb28.i.i.i.i.i.i ], [ %.pn32113.i.i.i.i.i.i, %bb36.i.i.i.i.i.i ]
; invoke <pyo3::internal::state::SuspendAttach as core::ops::drop::Drop>::drop
  invoke void @<pyo3::internal::state::SuspendAttach as core::ops::drop::Drop>::drop(ptr noalias nofree noundef nonnull align 8 dereferenceable(16) %_guard.i.i.i.i)
          to label %bb14.i.i.i unwind label %terminate.i.i.i.i, !noalias !ID

bb2.i.i.i.i:                                      ; preds = %<purrdf_native::py_store::quad_store::PyQuadStore>::query_governed::{closure#0}::{closure#0} (.exit.i.i.i.i.i)
  call void @llvm.lifetime.end.p0(ptr nonnull %parser_options.i.i.i.i.i.i), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_27.i.i.i.i.i.i), !noalias !ID
; invoke <pyo3::internal::state::SuspendAttach as core::ops::drop::Drop>::drop
  invoke void @<pyo3::internal::state::SuspendAttach as core::ops::drop::Drop>::drop(ptr noalias nofree noundef nonnull align 8 dereferenceable(16) %_guard.i.i.i.i)
          to label %bb3.i.i.i unwind label %cleanup2.i.i.i, !noalias !ID

terminate.i.i.i.i:                                ; preds = %bb6.i.i.i.i, %cleanup1.body.i.i.i.i
  %189 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #ATTR, !noalias !ID
  unreachable

bb6.i.i.i.i:                                      ; preds = %bb20.i.i.i
  %190 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<<purrdf_native::py_store::quad_store::PyQuadStore>::query_impl<()>::{closure#0}::{closure#0}>
  invoke fastcc void @core::ptr::drop_glue::<<purrdf_native::py_store::quad_store::PyQuadStore>::query_impl<()>::{closure#0}::{closure#0}>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(280) %_11.i.i.i)
          to label %bb14.i.i.i unwind label %terminate.i.i.i.i, !noalias !ID

bb14.i.i.i:                                       ; preds = %<alloc::alloc::Global as core::alloc::Allocator>::deallocate (.exit.i5.i.i.i.i.i.i.i.i.i), %cleanup.i.i.i.i.i.i.i.i.i, %cleanup3.i.i.i, %cleanup2.i.i.i, %bb6.i.i.i.i, %cleanup1.body.i.i.i.i
  %.pn.i.i.i = phi { ptr, i32 } [ %193, %cleanup3.i.i.i ], [ %eh.lpad-body.i.i.i.i, %cleanup1.body.i.i.i.i ], [ %190, %bb6.i.i.i.i ], [ %192, %cleanup2.i.i.i ], [ %205, %<alloc::alloc::Global as core::alloc::Allocator>::deallocate (.exit.i5.i.i.i.i.i.i.i.i.i) ], [ %205, %cleanup.i.i.i.i.i.i.i.i.i ]
  %_2.i.i.i.i.i = atomicrmw sub ptr %71, i64 1 release, align 8, !noalias !ID
  %191 = icmp eq i64 %_2.i.i.i.i.i, 1
  br i1 %191, label %bb2.i.i.i.i.i, label %bb15.i.i.i

bb2.i.i.i.i.i:                                    ; preds = %bb14.i.i.i
  fence acquire
; invoke <alloc::sync::Arc<purrdf_native::py_store::query::PyStopWatch>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_native::py_store::query::PyStopWatch>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %watch.i.i.i) #ATTR
          to label %bb15.i.i.i unwind label %terminate.i.i.i, !noalias !ID

cleanup2.i.i.i:                                   ; preds = %bb2.i.i.i.i.i.i.i.i.i.i.i.i, %bb2.i10.i.i.i, %bb2.i.i.i.i
  %192 = landingpad { ptr, i32 }
          cleanup
  br label %bb14.i.i.i

bb3.i.i.i:                                        ; preds = %bb2.i.i.i.i
  call void @llvm.lifetime.end.p0(ptr nonnull %_guard.i.i.i.i), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_11.i.i.i), !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %_13.i.i.i), !noalias !ID
  %_14.i.i.i = getelementptr inbounds nuw i8, ptr %71, i64 16
; invoke <purrdf_native::py_store::query::PyStopWatch>::take_interrupt
  invoke fastcc void @<purrdf_native::py_store::query::PyStopWatch>::take_interrupt(ptr noalias nofree noundef align 8 captures(none) dereferenceable(56) %_13.i.i.i, ptr noundef nonnull align 8 %_14.i.i.i)
          to label %bb4.i.i.i unwind label %cleanup3.i.i.i, !noalias !ID

cleanup3.i.i.i:                                   ; preds = %bb3.i.i.i
  %193 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<core::result::Result<purrdf_sparql_eval::governed::GovernedOutcome, pyo3::err::PyErr>>
  invoke fastcc void @core::ptr::drop_glue::<core::result::Result<purrdf_sparql_eval::governed::GovernedOutcome, pyo3::err::PyErr>>(ptr noalias nofree noundef align 8 dereferenceable(368) %outcome.i.i.i) #ATTR
          to label %bb14.i.i.i unwind label %terminate.i.i.i, !noalias !ID

bb4.i.i.i:                                        ; preds = %bb3.i.i.i
  %_15.i.i.i = load i64, ptr %_13.i.i.i, align 8, !range !ID, !noalias !ID, !noundef !ID
  %194 = trunc nuw i64 %_15.i.i.i to i1
  br i1 %194, label %bb5.i.i.i, label %bb6.i.i.i

bb5.i.i.i:                                        ; preds = %bb4.i.i.i
  %195 = getelementptr inbounds nuw i8, ptr %_13.i.i.i, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %_40.sroa.8.i.i, ptr noundef nonnull align 8 dereferenceable(48) %195, i64 48, i1 false), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_13.i.i.i), !noalias !ID
  %196 = load i64, ptr %outcome.i.i.i, align 8, !range !ID, !alias.scope !ID, !noalias !ID, !noundef !ID
  %.not.i.i.i.i = icmp eq i64 %196, -2
  br i1 %.not.i.i.i.i, label %bb3.i.i.i.i, label %bb2.i10.i.i.i

bb2.i10.i.i.i:                                    ; preds = %bb5.i.i.i
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::governed::GovernedOutcome>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::governed::GovernedOutcome>(ptr noalias nofree noundef nonnull align 8 dereferenceable(368) %outcome.i.i.i)
          to label %bb9.i.i.i unwind label %cleanup2.i.i.i, !noalias !ID

bb3.i.i.i.i:                                      ; preds = %bb5.i.i.i
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %197 = getelementptr inbounds nuw i8, ptr %outcome.i.i.i, i64 24
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %_2.i.i.i.i38.i.i.i = load i64, ptr %197, align 8, !range !ID, !alias.scope !ID, !noalias !ID, !noundef !ID
  %198 = icmp eq i64 %_2.i.i.i.i38.i.i.i, 0
  br i1 %198, label %bb9.i.i.i, label %bb2.i.i.i.i39.i.i.i

bb2.i.i.i.i39.i.i.i:                              ; preds = %bb3.i.i.i.i
  %199 = getelementptr inbounds nuw i8, ptr %outcome.i.i.i, i64 32
  %.val.i.i.i.i40.i.i.i = load ptr, ptr %199, align 8, !alias.scope !ID, !noalias !ID, !noundef !ID
  %200 = getelementptr inbounds nuw i8, ptr %outcome.i.i.i, i64 40
  %.val1.i.i.i.i.i.i.i = load ptr, ptr %200, align 8, !alias.scope !ID, !noalias !ID, !nonnull !ID, !noundef !ID
  %.not.i.i.i.i.i.i.i.i = icmp eq ptr %.val.i.i.i.i40.i.i.i, null
  br i1 %.not.i.i.i.i.i.i.i.i, label %bb3.i.i.i.i.i43.i.i.i, label %bb2.i.i.i.i.i41.i.i.i

bb2.i.i.i.i.i41.i.i.i:                            ; preds = %bb2.i.i.i.i39.i.i.i
  %201 = load ptr, ptr %.val1.i.i.i.i.i.i.i, align 8, !invariant.load !ID, !noalias !ID
  %.not.i.i.i.i.i.i.i.i.i = icmp eq ptr %201, null
  br i1 %.not.i.i.i.i.i.i.i.i.i, label %bb3.i.i.i.i.i.i.i.i.i, label %is_not_null.i.i.i.i.i.i.i.i.i

is_not_null.i.i.i.i.i.i.i.i.i:                    ; preds = %bb2.i.i.i.i.i41.i.i.i
  invoke void %201(ptr noundef nonnull %.val.i.i.i.i40.i.i.i)
          to label %bb3.i.i.i.i.i.i.i.i.i unwind label %cleanup.i.i.i.i.i.i.i.i.i, !noalias !ID

bb3.i.i.i.i.i.i.i.i.i:                            ; preds = %is_not_null.i.i.i.i.i.i.i.i.i, %bb2.i.i.i.i.i41.i.i.i
  %202 = getelementptr inbounds nuw i8, ptr %.val1.i.i.i.i.i.i.i, i64 8
  %size.i.i.i.i.i.i.i.i.i.i = load i64, ptr %202, align 8, !range !ID, !invariant.load !ID, !noalias !ID
  %203 = icmp eq i64 %size.i.i.i.i.i.i.i.i.i.i, 0
  br i1 %203, label %bb9.i.i.i, label %<alloc::alloc::Global as core::alloc::Allocator>::deallocate (.exit.i.i.i.i.i.i.i.i.i.i)

<alloc::alloc::Global as core::alloc::Allocator>::deallocate (.exit.i.i.i.i.i.i.i.i.i.i): ; preds = %bb3.i.i.i.i.i.i.i.i.i
  %204 = getelementptr inbounds nuw i8, ptr %.val1.i.i.i.i.i.i.i, i64 16
  %align.i.i.i.i.i.i.i.i.i.i = load i64, ptr %204, align 8, !range !ID, !invariant.load !ID, !noalias !ID
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %.val.i.i.i.i40.i.i.i, i64 noundef %size.i.i.i.i.i.i.i.i.i.i, i64 noundef range(i64 1, -9223372036854775807) %align.i.i.i.i.i.i.i.i.i.i) #ATTR, !noalias !ID
  br label %bb9.i.i.i

cleanup.i.i.i.i.i.i.i.i.i:                        ; preds = %is_not_null.i.i.i.i.i.i.i.i.i
  %205 = landingpad { ptr, i32 }
          cleanup
  %206 = getelementptr inbounds nuw i8, ptr %.val1.i.i.i.i.i.i.i, i64 8
  %size.i4.i.i.i.i.i.i.i.i.i = load i64, ptr %206, align 8, !range !ID, !invariant.load !ID, !noalias !ID
  %207 = icmp eq i64 %size.i4.i.i.i.i.i.i.i.i.i, 0
  br i1 %207, label %bb14.i.i.i, label %<alloc::alloc::Global as core::alloc::Allocator>::deallocate (.exit.i5.i.i.i.i.i.i.i.i.i)

<alloc::alloc::Global as core::alloc::Allocator>::deallocate (.exit.i5.i.i.i.i.i.i.i.i.i): ; preds = %cleanup.i.i.i.i.i.i.i.i.i
  %208 = getelementptr inbounds nuw i8, ptr %.val1.i.i.i.i.i.i.i, i64 16
  %align.i6.i.i.i.i.i.i.i.i.i = load i64, ptr %208, align 8, !range !ID, !invariant.load !ID, !noalias !ID
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %.val.i.i.i.i40.i.i.i, i64 noundef %size.i4.i.i.i.i.i.i.i.i.i, i64 noundef range(i64 1, -9223372036854775807) %align.i6.i.i.i.i.i.i.i.i.i) #ATTR, !noalias !ID
  br label %bb14.i.i.i

bb3.i.i.i.i.i43.i.i.i:                            ; preds = %bb2.i.i.i.i39.i.i.i
  %_3.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @pyo3::internal::state::ATTACH_COUNT::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL)
  %self3.val.i.i.i.i.i.i.i.i.i.i.i.i.i = load i64, ptr %_3.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, align 8, !noalias !ID, !noundef !ID
  %_0.i.i.i.i.i.i.i.i.i.i.i.i.i.i = icmp sgt i64 %self3.val.i.i.i.i.i.i.i.i.i.i.i.i.i, 0
  br i1 %_0.i.i.i.i.i.i.i.i.i.i.i.i.i.i, label %bb1.i.i.i.i.i.i.i.i.i.i.i.i, label %bb2.i.i.i.i.i.i.i.i.i.i.i.i, !prof !ID

bb2.i.i.i.i.i.i.i.i.i.i.i.i:                      ; preds = %bb3.i.i.i.i.i43.i.i.i
; invoke <pyo3::instance::Py<_> as core::ops::drop::Drop>::drop::drop_slow
  invoke void @<pyo3::instance::Py<_> as core::ops::drop::Drop>::drop::drop_slow(ptr noundef nonnull %.val1.i.i.i.i.i.i.i)
          to label %bb9.i.i.i unwind label %cleanup2.i.i.i, !noalias !ID

bb1.i.i.i.i.i.i.i.i.i.i.i.i:                      ; preds = %bb3.i.i.i.i.i43.i.i.i
  call void @_Py_DecRef(ptr noundef nonnull %.val1.i.i.i.i.i.i.i) #ATTR, !noalias !ID
  br label %bb9.i.i.i

bb6.i.i.i:                                        ; preds = %bb4.i.i.i
  call void @llvm.lifetime.end.p0(ptr nonnull %_13.i.i.i), !noalias !ID
  %_40.sroa.0.0.copyload.i.i = load i64, ptr %outcome.i.i.i, align 8, !noalias !ID
  %_40.sroa.8.0.outcome.i.sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %outcome.i.i.i, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %_40.sroa.8.i.i, ptr noundef nonnull align 8 dereferenceable(48) %_40.sroa.8.0.outcome.i.sroa_idx.i.i, i64 48, i1 false), !noalias !ID
  %_40.sroa.12.0.outcome.i.sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %outcome.i.i.i, i64 56
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(312) %_40.sroa.12.i.i, ptr noundef nonnull align 8 dereferenceable(312) %_40.sroa.12.0.outcome.i.sroa_idx.i.i, i64 312, i1 false), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %outcome.i.i.i), !noalias !ID
  %_2.i.i15.i.i.i = atomicrmw sub ptr %71, i64 1 release, align 8, !noalias !ID
  %209 = icmp eq i64 %_2.i.i15.i.i.i, 1
  br i1 %209, label %bb2.i.i16.i.i.i, label %bb7.i.i.i

bb2.i.i16.i.i.i:                                  ; preds = %bb6.i.i.i
  fence acquire
; invoke <alloc::sync::Arc<purrdf_native::py_store::query::PyStopWatch>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_native::py_store::query::PyStopWatch>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %watch.i.i.i) #ATTR
          to label %bb7.i.i.i unwind label %cleanup4.i.i.i, !noalias !ID

bb9.i.i.i:                                        ; preds = %bb1.i.i.i.i.i.i.i.i.i.i.i.i, %bb2.i.i.i.i.i.i.i.i.i.i.i.i, %<alloc::alloc::Global as core::alloc::Allocator>::deallocate (.exit.i.i.i.i.i.i.i.i.i.i), %bb3.i.i.i.i.i.i.i.i.i, %bb3.i.i.i.i, %bb2.i10.i.i.i
  call void @llvm.lifetime.end.p0(ptr nonnull %outcome.i.i.i), !noalias !ID
  %_2.i.i20.i.i.i = atomicrmw sub ptr %71, i64 1 release, align 8, !noalias !ID
  %210 = icmp eq i64 %_2.i.i20.i.i.i, 1
  br i1 %210, label %bb2.i.i21.i.i.i, label %bb10.i.i.i

bb2.i.i21.i.i.i:                                  ; preds = %bb9.i.i.i
  fence acquire
; invoke <alloc::sync::Arc<purrdf_native::py_store::query::PyStopWatch>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_native::py_store::query::PyStopWatch>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %watch.i.i.i) #ATTR
          to label %bb10.i.i.i unwind label %cleanup4.i.i.i, !noalias !ID

bb15.i.i.i:                                       ; preds = %cleanup4.i.i.i, %bb2.i.i.i.i.i, %bb14.i.i.i
  %.pn6.i.i.i = phi { ptr, i32 } [ %215, %cleanup4.i.i.i ], [ %.pn.i.i.i, %bb2.i.i.i.i.i ], [ %.pn.i.i.i, %bb14.i.i.i ]
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %211 = getelementptr inbounds nuw i8, ptr %governors.i.i.i, i64 64
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %212 = load ptr, ptr %211, align 8, !alias.scope !ID, !noalias !ID, !noundef !ID
  %213 = icmp eq ptr %212, null
  br i1 %213, label %bb47.i.i, label %bb2.i.i24.i.i.i

bb2.i.i24.i.i.i:                                  ; preds = %bb15.i.i.i
  %_2.i.i.i.i25.i.i.i = atomicrmw sub ptr %212, i64 1 release, align 8, !noalias !ID
  %214 = icmp eq i64 %_2.i.i.i.i25.i.i.i, 1
  br i1 %214, label %bb2.i.i.i.i26.i.i.i, label %bb47.i.i

bb2.i.i.i.i26.i.i.i:                              ; preds = %bb2.i.i24.i.i.i
  fence acquire
; invoke <alloc::sync::Arc<dyn purrdf_sparql_eval::governor::StopSignal>>::drop_slow
  invoke void @<alloc::sync::Arc<dyn purrdf_sparql_eval::governor::StopSignal>>::drop_slow(ptr noalias nofree noundef nonnull align 8 dereferenceable(16) %211) #ATTR
          to label %bb47.i.i unwind label %terminate.i.i.i, !noalias !ID

cleanup4.i.i.i:                                   ; preds = %bb2.i.i21.i.i.i, %bb2.i.i16.i.i.i
  %215 = landingpad { ptr, i32 }
          cleanup
  br label %bb15.i.i.i

bb10.i.i.i:                                       ; preds = %bb2.i.i21.i.i.i, %bb9.i.i.i
  call void @llvm.lifetime.end.p0(ptr nonnull %watch.i.i.i), !noalias !ID
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %216 = getelementptr inbounds nuw i8, ptr %governors.i.i.i, i64 64
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %217 = load ptr, ptr %216, align 8, !alias.scope !ID, !noalias !ID, !noundef !ID
  %218 = icmp eq ptr %217, null
  br i1 %218, label %bb11.i.i.i, label %bb2.i.i28.i.i.i

bb2.i.i28.i.i.i:                                  ; preds = %bb10.i.i.i
  %_2.i.i.i.i29.i.i.i = atomicrmw sub ptr %217, i64 1 release, align 8, !noalias !ID
  %219 = icmp eq i64 %_2.i.i.i.i29.i.i.i, 1
  br i1 %219, label %bb2.i.i.i.i30.i.i.i, label %bb11.i.i.i

bb2.i.i.i.i30.i.i.i:                              ; preds = %bb2.i.i28.i.i.i
  fence acquire
; invoke <alloc::sync::Arc<dyn purrdf_sparql_eval::governor::StopSignal>>::drop_slow
  invoke void @<alloc::sync::Arc<dyn purrdf_sparql_eval::governor::StopSignal>>::drop_slow(ptr noalias nofree noundef nonnull align 8 dereferenceable(16) %216) #ATTR
          to label %bb11.i.i.i unwind label %cleanup9.i.i, !noalias !ID

bb11.i.i.i:                                       ; preds = %bb2.i.i.i.i30.i.i.i, %bb2.i.i28.i.i.i, %bb10.i.i.i
  call void @llvm.lifetime.end.p0(ptr nonnull %governors.i.i.i), !noalias !ID
  br label %bb6.thread.i.i

bb7.i.i.i:                                        ; preds = %bb2.i.i16.i.i.i, %bb6.i.i.i
  call void @llvm.lifetime.end.p0(ptr nonnull %watch.i.i.i), !noalias !ID
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %220 = getelementptr inbounds nuw i8, ptr %governors.i.i.i, i64 64
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %221 = load ptr, ptr %220, align 8, !alias.scope !ID, !noalias !ID, !noundef !ID
  %222 = icmp eq ptr %221, null
  br i1 %222, label %bb6.i.i, label %bb2.i.i33.i.i.i

bb2.i.i33.i.i.i:                                  ; preds = %bb7.i.i.i
  %_2.i.i.i.i34.i.i.i = atomicrmw sub ptr %221, i64 1 release, align 8, !noalias !ID
  %223 = icmp eq i64 %_2.i.i.i.i34.i.i.i, 1
  br i1 %223, label %bb2.i.i.i.i35.i.i.i, label %bb6.i.i

bb2.i.i.i.i35.i.i.i:                              ; preds = %bb2.i.i33.i.i.i
  fence acquire
; invoke <alloc::sync::Arc<dyn purrdf_sparql_eval::governor::StopSignal>>::drop_slow
  invoke void @<alloc::sync::Arc<dyn purrdf_sparql_eval::governor::StopSignal>>::drop_slow(ptr noalias nofree noundef nonnull align 8 dereferenceable(16) %220) #ATTR
          to label %bb6.i.i unwind label %cleanup9.i.i, !noalias !ID

terminate.i.i.i:                                  ; preds = %bb17.i.i.i, %bb2.i.i.i.i26.i.i.i, %cleanup3.i.i.i, %bb2.i.i.i.i.i
  %224 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #ATTR, !noalias !ID
  unreachable

bb17.i.i.i:                                       ; preds = %bb58.i.i
  %lpad.thr_comm.split-lp.i.i.i = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<<purrdf_native::py_store::quad_store::PyQuadStore>::query_impl<()>::{closure#0}::{closure#0}>
  invoke fastcc void @core::ptr::drop_glue::<<purrdf_native::py_store::quad_store::PyQuadStore>::query_impl<()>::{closure#0}::{closure#0}>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(272) %_42.i.i) #ATTR
          to label %bb47.i.i unwind label %terminate.i.i.i, !noalias !ID

cleanup9.i.i:                                     ; preds = %bb60.i.i, %bb2.i.i.i.i35.i.i.i, %bb2.i.i.i.i30.i.i.i, %bb19.i.i.i
  %225 = landingpad { ptr, i32 }
          cleanup
  br label %bb47.i.i

bb6.thread.i.i:                                   ; preds = %bb11.i.i.i, %bb19.i.i.i
  call void @llvm.lifetime.end.p0(ptr nonnull %_42.i.i), !noalias !ID
  br label %bb59.i.i

bb6.i.i:                                          ; preds = %bb2.i.i.i.i35.i.i.i, %bb2.i.i33.i.i.i, %bb7.i.i.i
  call void @llvm.lifetime.end.p0(ptr nonnull %governors.i.i.i), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_42.i.i), !noalias !ID
  %226 = icmp eq i64 %_40.sroa.0.0.copyload.i.i, -2
  br i1 %226, label %bb59.i.i, label %bb60.i.i

bb59.i.i:                                         ; preds = %bb6.i.i, %bb6.thread.i.i
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %_39.sroa.6.i.i, ptr noundef nonnull align 8 dereferenceable(48) %_40.sroa.8.i.i, i64 48, i1 false), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_40.sroa.8.i.i)
  call void @llvm.lifetime.end.p0(ptr nonnull %_40.sroa.12.i.i)
  %227 = getelementptr inbounds nuw i8, ptr %_0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %227, ptr noundef nonnull align 8 dereferenceable(48) %_39.sroa.6.i.i, i64 48, i1 false), !noalias !ID
  store i64 1, ptr %_0, align 8, !alias.scope !ID, !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_39.sroa.6.i.i)
  call void @llvm.lifetime.end.p0(ptr nonnull %args.i.i), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %config.sroa.0.i.i)
  call void @llvm.lifetime.end.p0(ptr nonnull %specs.i.i), !noalias !ID
  br label %bb11.i.i

bb60.i.i:                                         ; preds = %bb6.i.i
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %_39.sroa.6.i.i, ptr noundef nonnull align 8 dereferenceable(48) %_40.sroa.8.i.i, i64 48, i1 false), !noalias !ID
  %_39.sroa.8.0.outcome.sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %outcome.i.i, i64 56
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(312) %_39.sroa.8.0.outcome.sroa_idx.i.i, ptr noundef nonnull align 8 dereferenceable(312) %_40.sroa.12.i.i, i64 312, i1 false), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_40.sroa.8.i.i)
  call void @llvm.lifetime.end.p0(ptr nonnull %_40.sroa.12.i.i)
  store i64 %_40.sroa.0.0.copyload.i.i, ptr %outcome.i.i, align 8, !noalias !ID
  %_39.sroa.6.0.outcome.sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %outcome.i.i, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %_39.sroa.6.0.outcome.sroa_idx.i.i, ptr noundef nonnull align 8 dereferenceable(48) %_39.sroa.6.i.i, i64 48, i1 false), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_39.sroa.6.i.i)
; invoke purrdf_native::py_store::query::materialize_outcome
  invoke fastcc void @purrdf_native::py_store::query::materialize_outcome(ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(56) %_0, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(368) %outcome.i.i)
          to label %bb7.i.i unwind label %cleanup9.i.i

bb7.i.i:                                          ; preds = %bb60.i.i
  call void @llvm.lifetime.end.p0(ptr nonnull %args.i.i), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %config.sroa.0.i.i)
  call void @llvm.lifetime.end.p0(ptr nonnull %specs.i.i), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %subs.i.i), !noalias !ID
  %228 = icmp sgt i64 %28, 0
  br i1 %228, label %bb2.i.i.i4.i.i.i50.i.i, label %<purrdf_native::py_store::quad_store::PyQuadStore>::query_governed::{closure#0} (.exit.i)

bb2.i.i.i4.i.i.i50.i.i:                           ; preds = %bb7.i.i
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %_59.i.i, i64 noundef %28, i64 noundef range(i64 1, -9223372036854775807) 1) #ATTR, !noalias !ID
  br label %<purrdf_native::py_store::quad_store::PyQuadStore>::query_governed::{closure#0} (.exit.i)

bb11.i.i:                                         ; preds = %bb62.i.i, %bb59.i.i
  %_48.sroa.0.4.i.i = phi i1 [ true, %bb62.i.i ], [ false, %bb59.i.i ]
  %_51.sroa.0.4.i.i = phi i8 [ %_51.sroa.0.5.i.i, %bb62.i.i ], [ 0, %bb59.i.i ]
  call void @llvm.lifetime.end.p0(ptr nonnull %subs.i.i), !noalias !ID
  %229 = icmp sgt i64 %28, 0
  br i1 %229, label %bb2.i.i.i4.i.i.i55.i.i, label %bb33.i.i

bb2.i.i.i4.i.i.i55.i.i:                           ; preds = %bb11.i.i
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %_59.i.i, i64 noundef %28, i64 noundef range(i64 1, -9223372036854775807) 1) #ATTR, !noalias !ID
  br label %bb33.i.i

bb8.i.i:                                          ; preds = %bb2.i.i.i4.i.i6.i.i.i.i, %bb4.i.i.i.i, %bb57.i.i
  %230 = icmp eq i64 %_23.i.sroa.0.0.copyload.i, -1
  br i1 %230, label %bb9.i.i, label %bb2.i59.i.i

bb2.i59.i.i:                                      ; preds = %bb8.i.i
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %_23.i.sroa.5.0.copyload.i) ]
  tail call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %_710.i.i.i.i.i.i = icmp eq i64 %_23.i.sroa.6.0.copyload.i, 0
  br i1 %_710.i.i.i.i.i.i, label %bb4.i.i63.i.i, label %bb5.i.i.i.i61.i.i

bb5.i.i.i.i61.i.i:                                ; preds = %bb2.i59.i.i, %core::ptr::drop_glue::<alloc::string::String> (.exit.i.i.i.i.i.i)
  %_3.sroa.0.011.i.i.i.i.i.i = phi i64 [ %231, %core::ptr::drop_glue::<alloc::string::String> (.exit.i.i.i.i.i.i) ], [ 0, %bb2.i59.i.i ]
  %_6.i.i.i.i62.i.i = getelementptr inbounds nuw [24 x i8], ptr %_23.i.sroa.5.0.copyload.i, i64 %_3.sroa.0.011.i.i.i.i.i.i
  %231 = add nuw nsw i64 %_3.sroa.0.011.i.i.i.i.i.i, 1
  tail call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %_1.val.i.i.i.i.i.i.i = load i64, ptr %_6.i.i.i.i62.i.i, align 8, !alias.scope !ID, !noalias !ID
  %232 = icmp eq i64 %_1.val.i.i.i.i.i.i.i, 0
  br i1 %232, label %core::ptr::drop_glue::<alloc::string::String> (.exit.i.i.i.i.i.i), label %bb2.i.i.i4.i.i.i.i.i.i.i.i

bb2.i.i.i4.i.i.i.i.i.i.i.i:                       ; preds = %bb5.i.i.i.i61.i.i
  %233 = getelementptr inbounds nuw i8, ptr %_6.i.i.i.i62.i.i, i64 8
  %_1.val1.i.i.i.i.i.i.i = load ptr, ptr %233, align 8, !alias.scope !ID, !noalias !ID, !nonnull !ID, !noundef !ID
; call __rustc::__rust_dealloc
  tail call void @__rustc::__rust_dealloc(ptr noundef nonnull %_1.val1.i.i.i.i.i.i.i, i64 noundef %_1.val.i.i.i.i.i.i.i, i64 noundef range(i64 1, -9223372036854775807) 1) #ATTR, !noalias !ID
  br label %core::ptr::drop_glue::<alloc::string::String> (.exit.i.i.i.i.i.i)

core::ptr::drop_glue::<alloc::string::String> (.exit.i.i.i.i.i.i): ; preds = %bb2.i.i.i4.i.i.i.i.i.i.i.i, %bb5.i.i.i.i61.i.i
  %_7.i.i.i.i.i.i = icmp eq i64 %231, %_23.i.sroa.6.0.copyload.i
  br i1 %_7.i.i.i.i.i.i, label %bb4.i.i63.i.i, label %bb5.i.i.i.i61.i.i

bb4.i.i63.i.i:                                    ; preds = %core::ptr::drop_glue::<alloc::string::String> (.exit.i.i.i.i.i.i), %bb2.i59.i.i
  %234 = icmp eq i64 %_23.i.sroa.0.0.copyload.i, 0
  br i1 %234, label %bb9.i.i, label %bb2.i.i.i6.i.i.i.i

bb2.i.i.i6.i.i.i.i:                               ; preds = %bb4.i.i63.i.i
  %alloc_size.i.i.i.i7.i.i.i.i = mul nuw i64 %_23.i.sroa.0.0.copyload.i, 24
; call __rustc::__rust_dealloc
  tail call void @__rustc::__rust_dealloc(ptr noundef nonnull %_23.i.sroa.5.0.copyload.i, i64 noundef %alloc_size.i.i.i.i7.i.i.i.i, i64 noundef range(i64 1, -9223372036854775807) 8) #ATTR, !noalias !ID
  br label %bb9.i.i

bb9.i.i:                                          ; preds = %bb2.i.i.i6.i.i.i.i, %bb4.i.i63.i.i, %bb8.i.i
  %235 = icmp eq i64 %_22.i.sroa.0.0.copyload.i, -1
  br i1 %235, label %bb10.i.i, label %bb2.i65.i.i

bb2.i65.i.i:                                      ; preds = %bb9.i.i
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %_22.i.sroa.5.0.copyload.i) ]
  tail call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %_710.i.i.i.i68.i.i = icmp eq i64 %_22.i.sroa.6.0.copyload.i, 0
  br i1 %_710.i.i.i.i68.i.i, label %bb4.i.i77.i.i, label %bb5.i.i.i.i69.i.i

bb5.i.i.i.i69.i.i:                                ; preds = %bb2.i65.i.i, %core::ptr::drop_glue::<alloc::string::String> (.exit.i.i.i.i75.i.i)
  %_3.sroa.0.011.i.i.i.i70.i.i = phi i64 [ %236, %core::ptr::drop_glue::<alloc::string::String> (.exit.i.i.i.i75.i.i) ], [ 0, %bb2.i65.i.i ]
  %_6.i.i.i.i71.i.i = getelementptr inbounds nuw [24 x i8], ptr %_22.i.sroa.5.0.copyload.i, i64 %_3.sroa.0.011.i.i.i.i70.i.i
  %236 = add nuw nsw i64 %_3.sroa.0.011.i.i.i.i70.i.i, 1
  tail call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %_1.val.i.i.i.i.i72.i.i = load i64, ptr %_6.i.i.i.i71.i.i, align 8, !alias.scope !ID, !noalias !ID
  %237 = icmp eq i64 %_1.val.i.i.i.i.i72.i.i, 0
  br i1 %237, label %core::ptr::drop_glue::<alloc::string::String> (.exit.i.i.i.i75.i.i), label %bb2.i.i.i4.i.i.i.i.i.i73.i.i

bb2.i.i.i4.i.i.i.i.i.i73.i.i:                     ; preds = %bb5.i.i.i.i69.i.i
  %238 = getelementptr inbounds nuw i8, ptr %_6.i.i.i.i71.i.i, i64 8
  %_1.val1.i.i.i.i.i74.i.i = load ptr, ptr %238, align 8, !alias.scope !ID, !noalias !ID, !nonnull !ID, !noundef !ID
; call __rustc::__rust_dealloc
  tail call void @__rustc::__rust_dealloc(ptr noundef nonnull %_1.val1.i.i.i.i.i74.i.i, i64 noundef %_1.val.i.i.i.i.i72.i.i, i64 noundef range(i64 1, -9223372036854775807) 1) #ATTR, !noalias !ID
  br label %core::ptr::drop_glue::<alloc::string::String> (.exit.i.i.i.i75.i.i)

core::ptr::drop_glue::<alloc::string::String> (.exit.i.i.i.i75.i.i): ; preds = %bb2.i.i.i4.i.i.i.i.i.i73.i.i, %bb5.i.i.i.i69.i.i
  %_7.i.i.i.i76.i.i = icmp eq i64 %236, %_22.i.sroa.6.0.copyload.i
  br i1 %_7.i.i.i.i76.i.i, label %bb4.i.i77.i.i, label %bb5.i.i.i.i69.i.i

bb4.i.i77.i.i:                                    ; preds = %core::ptr::drop_glue::<alloc::string::String> (.exit.i.i.i.i75.i.i), %bb2.i65.i.i
  %239 = icmp eq i64 %_22.i.sroa.0.0.copyload.i, 0
  br i1 %239, label %bb10.i.i, label %bb2.i.i.i6.i.i78.i.i

bb2.i.i.i6.i.i78.i.i:                             ; preds = %bb4.i.i77.i.i
  %alloc_size.i.i.i.i7.i.i79.i.i = mul nuw i64 %_22.i.sroa.0.0.copyload.i, 24
; call __rustc::__rust_dealloc
  tail call void @__rustc::__rust_dealloc(ptr noundef nonnull %_22.i.sroa.5.0.copyload.i, i64 noundef %alloc_size.i.i.i.i7.i.i79.i.i, i64 noundef range(i64 1, -9223372036854775807) 8) #ATTR, !noalias !ID
  br label %bb10.i.i

bb10.i.i:                                         ; preds = %bb2.i.i.i6.i.i78.i.i, %bb4.i.i77.i.i, %bb9.i.i
  call void @llvm.lifetime.end.p0(ptr nonnull %config.sroa.0.i.i)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %240 = getelementptr inbounds nuw i8, ptr %specs.i.i, i64 8
  %_1.val.i.i.i = load ptr, ptr %240, align 8, !alias.scope !ID, !noalias !ID, !nonnull !ID, !noundef !ID
  %241 = getelementptr inbounds nuw i8, ptr %specs.i.i, i64 16
  %_1.val1.i.i.i = load i64, ptr %241, align 8, !alias.scope !ID, !noalias !ID, !noundef !ID
  %_7.i.i.i.i.i32 = icmp eq i64 %_1.val1.i.i.i, 0
  br i1 %_7.i.i.i.i.i32, label %bb4.i83.i.i, label %bb5.i.i.i.i.i

bb6.i.i.i.i.i:                                    ; preds = %bb5.i.i.i.i.i
  %_7.i.i.i.i.i = icmp eq i64 %242, %_1.val1.i.i.i
  br i1 %_7.i.i.i.i.i, label %bb4.i83.i.i, label %bb5.i.i.i.i.i

bb5.i.i.i.i.i:                                    ; preds = %bb10.i.i, %bb6.i.i.i.i.i
  %_3.sroa.0.0.i.i.i.i.i33 = phi i64 [ %242, %bb6.i.i.i.i.i ], [ 0, %bb10.i.i ]
  %_6.i.i.i.i.i = getelementptr inbounds nuw [160 x i8], ptr %_1.val.i.i.i, i64 %_3.sroa.0.0.i.i.i.i.i33
  %242 = add nuw nsw i64 %_3.sroa.0.0.i.i.i.i.i33, 1
; invoke core::ptr::drop_glue::<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>
  invoke fastcc void @core::ptr::drop_glue::<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>(ptr noalias nofree noundef align 8 dereferenceable(160) %_6.i.i.i.i.i)
          to label %bb6.i.i.i.i.i unwind label %cleanup.i.i.i.i.i, !noalias !ID

bb4.i.i.i.i.i:                                    ; preds = %bb3.i.i.i.i.i
  %243 = add i64 %_3.sroa.0.1.i.i.i.i.i35, 1
  %_5.i.i.i.i.i = icmp eq i64 %243, %_1.val1.i.i.i
  br i1 %_5.i.i.i.i.i, label %cleanup.body.i.i.i, label %bb3.i.i.i.i.i

cleanup.i.i.i.i.i:                                ; preds = %bb5.i.i.i.i.i
  %244 = landingpad { ptr, i32 }
          cleanup
  %_5.i.i.i.i.i34 = icmp eq i64 %242, %_1.val1.i.i.i
  br i1 %_5.i.i.i.i.i34, label %cleanup.body.i.i.i, label %bb3.i.i.i.i.i

bb3.i.i.i.i.i:                                    ; preds = %cleanup.i.i.i.i.i, %bb4.i.i.i.i.i
  %_3.sroa.0.1.i.i.i.i.i35 = phi i64 [ %243, %bb4.i.i.i.i.i ], [ %242, %cleanup.i.i.i.i.i ]
  %_4.i.i.i.i.i = getelementptr inbounds nuw [160 x i8], ptr %_1.val.i.i.i, i64 %_3.sroa.0.1.i.i.i.i.i35
; invoke core::ptr::drop_glue::<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>
  invoke fastcc void @core::ptr::drop_glue::<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>(ptr noalias nofree noundef align 8 dereferenceable(160) %_4.i.i.i.i.i) #ATTR
          to label %bb4.i.i.i.i.i unwind label %terminate.i.i.i.i.i, !noalias !ID

terminate.i.i.i.i.i:                              ; preds = %bb3.i.i.i.i.i
  %245 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  tail call void @core::panicking::panic_in_cleanup() #ATTR, !noalias !ID
  unreachable

cleanup.body.i.i.i:                               ; preds = %bb4.i.i.i.i.i, %cleanup.i.i.i.i.i
  %_1.val2.i.i.i = load i64, ptr %specs.i.i, align 8, !alias.scope !ID, !noalias !ID
  %246 = icmp eq i64 %_1.val2.i.i.i, 0
  br i1 %246, label %bb37.i.i, label %bb2.i.i.i.i.i.i

bb2.i.i.i.i.i.i:                                  ; preds = %cleanup.body.i.i.i
  %alloc_size.i.i.i.i.i.i.i = mul nuw i64 %_1.val2.i.i.i, 160
; call __rustc::__rust_dealloc
  tail call void @__rustc::__rust_dealloc(ptr noundef nonnull %_1.val.i.i.i, i64 noundef %alloc_size.i.i.i.i.i.i.i, i64 noundef range(i64 1, -9223372036854775807) 8) #ATTR, !noalias !ID
  br label %bb37.i.i

bb4.i83.i.i:                                      ; preds = %bb6.i.i.i.i.i, %bb10.i.i
  %_1.val4.i.i.i = load i64, ptr %specs.i.i, align 8, !alias.scope !ID, !noalias !ID
  %247 = icmp eq i64 %_1.val4.i.i.i, 0
  br i1 %247, label %bb62.i.i, label %bb2.i.i.i6.i.i.i

bb2.i.i.i6.i.i.i:                                 ; preds = %bb4.i83.i.i
  %alloc_size.i.i.i.i7.i.i.i = mul nuw i64 %_1.val4.i.i.i, 160
; call __rustc::__rust_dealloc
  tail call void @__rustc::__rust_dealloc(ptr noundef nonnull %_1.val.i.i.i, i64 noundef %alloc_size.i.i.i.i7.i.i.i, i64 noundef range(i64 1, -9223372036854775807) 8) #ATTR, !noalias !ID
  br label %bb62.i.i

bb62.i.i:                                         ; preds = %bb2.i.i.i6.i.i.i, %bb4.i83.i.i, %bb55.i.i
  %_51.sroa.0.5.i.i = phi i8 [ 1, %bb55.i.i ], [ 0, %bb4.i83.i.i ], [ 0, %bb2.i.i.i6.i.i.i ]
  call void @llvm.lifetime.end.p0(ptr nonnull %specs.i.i), !noalias !ID
; invoke core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_core::ir::term::TermValue)>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_core::ir::term::TermValue)>>(ptr noalias nofree noundef align 8 dereferenceable(24) %subs.i.i)
          to label %bb11.i.i unwind label %cleanup.i.i, !noalias !ID

terminate.i.i:                                    ; preds = %bb37.i.i, %cleanup8.i.i
  %248 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  tail call void @core::panicking::panic_in_cleanup() #ATTR, !noalias !ID
  unreachable

bb33.i.i:                                         ; preds = %bb2.i.i.i4.i.i.i55.i.i, %bb11.i.i
  %249 = trunc nuw i8 %_51.sroa.0.4.i.i to i1
  br i1 %249, label %bb34.i.i, label %bb27.i.i

bb34.i.i:                                         ; preds = %bb2.i.i.i4.i.i.i103.i.i, %bb61.i.i, %bb33.i.i
  %_48.sroa.0.7.i.i = phi i1 [ %_48.sroa.0.4.i.i, %bb33.i.i ], [ true, %bb61.i.i ], [ true, %bb2.i.i.i4.i.i.i103.i.i ]
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %250 = load i64, ptr %5, align 8, !range !ID, !alias.scope !ID, !noalias !ID, !noundef !ID
  %251 = icmp eq i64 %250, -1
  br i1 %251, label %bb32.i.i, label %bb2.i85.i.i

bb2.i85.i.i:                                      ; preds = %bb34.i.i
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %252 = getelementptr inbounds nuw i8, ptr %_22, i64 128
  %_1.val.i.i86.i.i = load ptr, ptr %252, align 8, !alias.scope !ID, !noalias !ID, !nonnull !ID, !noundef !ID
  %253 = getelementptr inbounds nuw i8, ptr %_22, i64 136
  %_1.val1.i.i87.i.i = load i64, ptr %253, align 8, !alias.scope !ID, !noalias !ID, !noundef !ID
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %_710.i.i.i.i88.i.i = icmp eq i64 %_1.val1.i.i87.i.i, 0
  br i1 %_710.i.i.i.i88.i.i, label %bb4.i.i97.i.i, label %bb5.i.i.i.i89.i.i

bb5.i.i.i.i89.i.i:                                ; preds = %bb2.i85.i.i, %core::ptr::drop_glue::<alloc::string::String> (.exit.i.i.i.i95.i.i)
  %_3.sroa.0.011.i.i.i.i90.i.i = phi i64 [ %254, %core::ptr::drop_glue::<alloc::string::String> (.exit.i.i.i.i95.i.i) ], [ 0, %bb2.i85.i.i ]
  %_6.i.i.i.i91.i.i = getelementptr inbounds nuw [24 x i8], ptr %_1.val.i.i86.i.i, i64 %_3.sroa.0.011.i.i.i.i90.i.i
  %254 = add nuw nsw i64 %_3.sroa.0.011.i.i.i.i90.i.i, 1
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %_1.val.i.i.i.i.i92.i.i = load i64, ptr %_6.i.i.i.i91.i.i, align 8, !alias.scope !ID, !noalias !ID
  %255 = icmp eq i64 %_1.val.i.i.i.i.i92.i.i, 0
  br i1 %255, label %core::ptr::drop_glue::<alloc::string::String> (.exit.i.i.i.i95.i.i), label %bb2.i.i.i4.i.i.i.i.i.i93.i.i

bb2.i.i.i4.i.i.i.i.i.i93.i.i:                     ; preds = %bb5.i.i.i.i89.i.i
  %256 = getelementptr inbounds nuw i8, ptr %_6.i.i.i.i91.i.i, i64 8
  %_1.val1.i.i.i.i.i94.i.i = load ptr, ptr %256, align 8, !alias.scope !ID, !noalias !ID, !nonnull !ID, !noundef !ID
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %_1.val1.i.i.i.i.i94.i.i, i64 noundef %_1.val.i.i.i.i.i92.i.i, i64 noundef range(i64 1, -9223372036854775807) 1) #ATTR, !noalias !ID
  br label %core::ptr::drop_glue::<alloc::string::String> (.exit.i.i.i.i95.i.i)

core::ptr::drop_glue::<alloc::string::String> (.exit.i.i.i.i95.i.i): ; preds = %bb2.i.i.i4.i.i.i.i.i.i93.i.i, %bb5.i.i.i.i89.i.i
  %_7.i.i.i.i96.i.i = icmp eq i64 %254, %_1.val1.i.i87.i.i
  br i1 %_7.i.i.i.i96.i.i, label %bb4.i.i97.i.i, label %bb5.i.i.i.i89.i.i

bb4.i.i97.i.i:                                    ; preds = %core::ptr::drop_glue::<alloc::string::String> (.exit.i.i.i.i95.i.i), %bb2.i85.i.i
  %257 = icmp eq i64 %250, 0
  br i1 %257, label %bb32.i.i, label %bb2.i.i.i6.i.i98.i.i

bb2.i.i.i6.i.i98.i.i:                             ; preds = %bb4.i.i97.i.i
  %alloc_size.i.i.i.i7.i.i99.i.i = mul nuw i64 %250, 24
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %_1.val.i.i86.i.i, i64 noundef %alloc_size.i.i.i.i7.i.i99.i.i, i64 noundef range(i64 1, -9223372036854775807) 8) #ATTR, !noalias !ID
  br label %bb32.i.i

bb37.i.i:                                         ; preds = %bb2.i.i.i.i.i.i, %cleanup.body.i.i.i, %cleanup8.i.i, %cleanup7.i.i
  %_51.sroa.0.2.ph.i.i = phi i8 [ 0, %cleanup8.i.i ], [ 1, %cleanup7.i.i ], [ 0, %bb2.i.i.i.i.i.i ], [ 0, %cleanup.body.i.i.i ]
  %.pn33.ph.i.i = phi { ptr, i32 } [ %45, %cleanup8.i.i ], [ %41, %cleanup7.i.i ], [ %244, %bb2.i.i.i.i.i.i ], [ %244, %cleanup.body.i.i.i ]
; invoke core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_core::ir::term::TermValue)>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_core::ir::term::TermValue)>>(ptr noalias nofree noundef align 8 dereferenceable(24) %subs.i.i) #ATTR
          to label %bb47.i.i unwind label %terminate.i.i, !noalias !ID

bb61.i.i:                                         ; preds = %bb53.i.i, %bb51.i.i
  %258 = icmp sgt i64 %28, 0
  br i1 %258, label %bb2.i.i.i4.i.i.i103.i.i, label %bb34.i.i

bb2.i.i.i4.i.i.i103.i.i:                          ; preds = %bb61.i.i
; call __rustc::__rust_dealloc
  tail call void @__rustc::__rust_dealloc(ptr noundef nonnull %_59.i.i, i64 noundef %28, i64 noundef range(i64 1, -9223372036854775807) 1) #ATTR, !noalias !ID
  br label %bb34.i.i

bb32.i.i:                                         ; preds = %bb2.i.i.i6.i.i98.i.i, %bb4.i.i97.i.i, %bb34.i.i
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %259 = load i64, ptr %6, align 8, !range !ID, !alias.scope !ID, !noalias !ID, !noundef !ID
  %260 = icmp eq i64 %259, -1
  br i1 %260, label %bb30.i.i, label %bb2.i107.i.i

bb2.i107.i.i:                                     ; preds = %bb32.i.i
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %261 = getelementptr inbounds nuw i8, ptr %_22, i64 152
  %_1.val.i.i108.i.i = load ptr, ptr %261, align 8, !alias.scope !ID, !noalias !ID, !nonnull !ID, !noundef !ID
  %262 = getelementptr inbounds nuw i8, ptr %_22, i64 160
  %_1.val1.i.i109.i.i = load i64, ptr %262, align 8, !alias.scope !ID, !noalias !ID, !noundef !ID
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %_710.i.i.i.i110.i.i = icmp eq i64 %_1.val1.i.i109.i.i, 0
  br i1 %_710.i.i.i.i110.i.i, label %bb4.i.i119.i.i, label %bb5.i.i.i.i111.i.i

bb5.i.i.i.i111.i.i:                               ; preds = %bb2.i107.i.i, %core::ptr::drop_glue::<alloc::string::String> (.exit.i.i.i.i117.i.i)
  %_3.sroa.0.011.i.i.i.i112.i.i = phi i64 [ %263, %core::ptr::drop_glue::<alloc::string::String> (.exit.i.i.i.i117.i.i) ], [ 0, %bb2.i107.i.i ]
  %_6.i.i.i.i113.i.i = getelementptr inbounds nuw [24 x i8], ptr %_1.val.i.i108.i.i, i64 %_3.sroa.0.011.i.i.i.i112.i.i
  %263 = add nuw nsw i64 %_3.sroa.0.011.i.i.i.i112.i.i, 1
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %_1.val.i.i.i.i.i114.i.i = load i64, ptr %_6.i.i.i.i113.i.i, align 8, !alias.scope !ID, !noalias !ID
  %264 = icmp eq i64 %_1.val.i.i.i.i.i114.i.i, 0
  br i1 %264, label %core::ptr::drop_glue::<alloc::string::String> (.exit.i.i.i.i117.i.i), label %bb2.i.i.i4.i.i.i.i.i.i115.i.i

bb2.i.i.i4.i.i.i.i.i.i115.i.i:                    ; preds = %bb5.i.i.i.i111.i.i
  %265 = getelementptr inbounds nuw i8, ptr %_6.i.i.i.i113.i.i, i64 8
  %_1.val1.i.i.i.i.i116.i.i = load ptr, ptr %265, align 8, !alias.scope !ID, !noalias !ID, !nonnull !ID, !noundef !ID
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %_1.val1.i.i.i.i.i116.i.i, i64 noundef %_1.val.i.i.i.i.i114.i.i, i64 noundef range(i64 1, -9223372036854775807) 1) #ATTR, !noalias !ID
  br label %core::ptr::drop_glue::<alloc::string::String> (.exit.i.i.i.i117.i.i)

core::ptr::drop_glue::<alloc::string::String> (.exit.i.i.i.i117.i.i): ; preds = %bb2.i.i.i4.i.i.i.i.i.i115.i.i, %bb5.i.i.i.i111.i.i
  %_7.i.i.i.i118.i.i = icmp eq i64 %263, %_1.val1.i.i109.i.i
  br i1 %_7.i.i.i.i118.i.i, label %bb4.i.i119.i.i, label %bb5.i.i.i.i111.i.i

bb4.i.i119.i.i:                                   ; preds = %core::ptr::drop_glue::<alloc::string::String> (.exit.i.i.i.i117.i.i), %bb2.i107.i.i
  %266 = icmp eq i64 %259, 0
  br i1 %266, label %bb30.i.i, label %bb2.i.i.i6.i.i120.i.i

bb2.i.i.i6.i.i120.i.i:                            ; preds = %bb4.i.i119.i.i
  %alloc_size.i.i.i.i7.i.i121.i.i = mul nuw i64 %259, 24
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %_1.val.i.i108.i.i, i64 noundef %alloc_size.i.i.i.i7.i.i121.i.i, i64 noundef range(i64 1, -9223372036854775807) 8) #ATTR, !noalias !ID
  br label %bb30.i.i

bb27.i.i:                                         ; preds = %bb4.i.i127.i.i, %bb30.i.i, %bb33.i.i
  %_48.sroa.0.6162.i.i = phi i1 [ %_48.sroa.0.7.i.i, %bb30.i.i ], [ %_48.sroa.0.7.i.i, %bb4.i.i127.i.i ], [ %_48.sroa.0.4.i.i, %bb33.i.i ]
  br i1 %_48.sroa.0.6162.i.i, label %bb28.i.i, label %<purrdf_native::py_store::quad_store::PyQuadStore>::query_governed::{closure#0} (.exit.i)

bb30.i.i:                                         ; preds = %bb2.i.i.i6.i.i120.i.i, %bb4.i.i119.i.i, %bb32.i.i
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %267 = load i64, ptr %7, align 8, !range !ID, !alias.scope !ID, !noalias !ID, !noundef !ID
  %268 = icmp eq i64 %267, -1
  br i1 %268, label %bb27.i.i, label %bb2.i124.i.i

bb2.i124.i.i:                                     ; preds = %bb30.i.i
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %269 = icmp eq i64 %267, 0
  br i1 %269, label %bb4.i.i127.i.i, label %bb2.i.i.i4.i.i.i.i125.i.i

bb2.i.i.i4.i.i.i.i125.i.i:                        ; preds = %bb2.i124.i.i
  %270 = getelementptr inbounds nuw i8, ptr %_22, i64 176
  %_1.val1.i.i.i126.i.i = load ptr, ptr %270, align 8, !alias.scope !ID, !noalias !ID, !nonnull !ID, !noundef !ID
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %_1.val1.i.i.i126.i.i, i64 noundef %267, i64 noundef range(i64 1, -9223372036854775807) 1) #ATTR, !noalias !ID
  br label %bb4.i.i127.i.i

bb4.i.i127.i.i:                                   ; preds = %bb2.i.i.i4.i.i.i.i125.i.i, %bb2.i124.i.i
  %271 = getelementptr inbounds nuw i8, ptr %_22, i64 192
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %_1.val.i5.i.i128.i.i = load i64, ptr %271, align 8, !alias.scope !ID, !noalias !ID
  %272 = icmp eq i64 %_1.val.i5.i.i128.i.i, 0
  br i1 %272, label %bb27.i.i, label %bb2.i.i.i4.i.i6.i.i129.i.i

bb2.i.i.i4.i.i6.i.i129.i.i:                       ; preds = %bb4.i.i127.i.i
  %273 = getelementptr inbounds nuw i8, ptr %_22, i64 200
  %_1.val1.i7.i.i130.i.i = load ptr, ptr %273, align 8, !alias.scope !ID, !noalias !ID, !nonnull !ID, !noundef !ID
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %_1.val1.i7.i.i130.i.i, i64 noundef %_1.val.i5.i.i128.i.i, i64 noundef range(i64 1, -9223372036854775807) 1) #ATTR, !noalias !ID
  br i1 %_48.sroa.0.7.i.i, label %bb28.i.i, label %<purrdf_native::py_store::quad_store::PyQuadStore>::query_governed::{closure#0} (.exit.i)

bb28.i.i:                                         ; preds = %bb2.i.i.i4.i.i6.i.i129.i.i, %bb27.i.i
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %274 = load i64, ptr %25, align 8, !range !ID, !alias.scope !ID, !noalias !ID, !noundef !ID
  %275 = icmp eq i64 %274, -1
  br i1 %275, label %<purrdf_native::py_store::quad_store::PyQuadStore>::query_governed::{closure#0} (.exit.i), label %bb2.i133.i.i

bb2.i133.i.i:                                     ; preds = %bb28.i.i
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %276 = icmp eq i64 %274, 0
  br i1 %276, label %<purrdf_native::py_store::quad_store::PyQuadStore>::query_governed::{closure#0} (.exit.i), label %bb2.i.i.i4.i.i.i134.i.i

bb2.i.i.i4.i.i.i134.i.i:                          ; preds = %bb2.i133.i.i
  %277 = getelementptr inbounds nuw i8, ptr %_22, i64 224
  %_1.val1.i.i135.i.i = load ptr, ptr %277, align 8, !alias.scope !ID, !noalias !ID, !nonnull !ID, !noundef !ID
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %_1.val1.i.i135.i.i, i64 noundef %274, i64 noundef range(i64 1, -9223372036854775807) 1) #ATTR, !noalias !ID
  br label %<purrdf_native::py_store::quad_store::PyQuadStore>::query_governed::{closure#0} (.exit.i)

common.resume.i:                                  ; preds = %bb6.i2.i, %cleanup1.body.i.i, %bb2.i.i.i4.i.i.i144.i.i, %bb2.i143.i.i, %bb40.i.i, %bb39.i.i
  %common.resume.op.i = phi { ptr, i32 } [ %.pn35.i.i, %bb39.i.i ], [ %.pn35.i.i, %bb2.i.i.i4.i.i.i144.i.i ], [ %.pn35.i.i, %bb2.i143.i.i ], [ %.pn35.i.i, %bb40.i.i ], [ %316, %bb6.i2.i ], [ %eh.lpad-body.i.i, %cleanup1.body.i.i ]
  resume { ptr, i32 } %common.resume.op.i

bb45.i.i:                                         ; preds = %bb2.i.i.i4.i.i.i.i.i, %bb47.i.i
  %cond44.i.i = icmp eq i8 %_51.sroa.0.0.i.i, 0
  br i1 %cond44.i.i, label %bb39.i.i, label %bb46.i.i

bb46.i.i:                                         ; preds = %bb45.i.i
; call core::ptr::drop_glue::<core::option::Option<alloc::vec::Vec<alloc::string::String>>>
  call fastcc void @core::ptr::drop_glue::<core::option::Option<alloc::vec::Vec<alloc::string::String>>>(ptr noalias nofree noundef readonly align 8 dereferenceable(24) %5) #ATTR, !noalias !ID
; call core::ptr::drop_glue::<core::option::Option<alloc::vec::Vec<alloc::string::String>>>
  call fastcc void @core::ptr::drop_glue::<core::option::Option<alloc::vec::Vec<alloc::string::String>>>(ptr noalias nofree noundef readonly align 8 dereferenceable(24) %6) #ATTR, !noalias !ID
; call core::ptr::drop_glue::<core::option::Option<purrdf_sparql_results::model::ProvenanceNamespace>>
  call fastcc void @core::ptr::drop_glue::<core::option::Option<purrdf_sparql_results::model::ProvenanceNamespace>>(ptr noalias nofree noundef readonly align 8 dereferenceable(48) %7) #ATTR, !noalias !ID
  br label %bb39.i.i

bb39.i.i:                                         ; preds = %bb46.i.i, %bb45.i.i
  br i1 %_48.sroa.0.0.i.i, label %bb40.i.i, label %common.resume.i

bb40.i.i:                                         ; preds = %bb39.i.i
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %278 = load i64, ptr %25, align 8, !range !ID, !alias.scope !ID, !noalias !ID, !noundef !ID
  %279 = icmp eq i64 %278, -1
  br i1 %279, label %common.resume.i, label %bb2.i143.i.i

bb2.i143.i.i:                                     ; preds = %bb40.i.i
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %280 = icmp eq i64 %278, 0
  br i1 %280, label %common.resume.i, label %bb2.i.i.i4.i.i.i144.i.i

bb2.i.i.i4.i.i.i144.i.i:                          ; preds = %bb2.i143.i.i
  %281 = getelementptr inbounds nuw i8, ptr %_22, i64 224
  %_1.val1.i.i145.i.i = load ptr, ptr %281, align 8, !alias.scope !ID, !noalias !ID, !nonnull !ID, !noundef !ID
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %_1.val1.i.i145.i.i, i64 noundef %278, i64 noundef range(i64 1, -9223372036854775807) 1) #ATTR, !noalias !ID
  br label %common.resume.i

<purrdf_native::py_store::quad_store::PyQuadStore>::query_governed::{closure#0} (.exit.i): ; preds = %bb2.i.i.i4.i.i.i134.i.i, %bb2.i133.i.i, %bb28.i.i, %bb2.i.i.i4.i.i6.i.i129.i.i, %bb27.i.i, %bb2.i.i.i4.i.i.i50.i.i, %bb7.i.i
  call void @llvm.lifetime.end.p0(ptr nonnull %_26.i.i), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %outcome.i.i), !noalias !ID
  %_2.i = load i64, ptr %_0, align 8, !range !ID, !alias.scope !ID, !noalias !ID, !noundef !ID
  %282 = trunc nuw i64 %_2.i to i1
  br i1 %282, label %bb4.i, label %purrdf_native::py_store::presentation::settled::<pyo3::instance::Py<purrdf_native::py_store::query::PyQueryOutcome>, <purrdf_native::py_store::quad_store::PyQuadStore>::query_governed::{closure#0}> (.exit)

bb4.i:                                            ; preds = %<purrdf_native::py_store::quad_store::PyQuadStore>::query_governed::{closure#0} (.exit.i)
  %283 = getelementptr inbounds nuw i8, ptr %_0, i64 8
  call void @llvm.lifetime.start.p0(ptr nonnull %_4.sroa.0.i)
  call void @llvm.lifetime.start.p0(ptr nonnull %guard.i.i), !noalias !ID
; invoke <pyo3::internal::state::AttachGuard>::attach
  %284 = invoke noundef i32 @<pyo3::internal::state::AttachGuard>::attach()
          to label %bb1.i5.i unwind label %bb6.i2.i, !noalias !ID

bb1.i5.i:                                         ; preds = %bb4.i
  store i32 %284, ptr %guard.i.i, align 4, !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %_3.i1.i), !noalias !ID
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %_3.i1.i, ptr noundef nonnull align 8 dereferenceable(48) %283, i64 48, i1 false), !noalias !ID
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %_2.i.i.i.i.i.i = load ptr, ptr @PyExc_ValueError, align 8, !noalias !ID, !nonnull !ID, !noundef !ID
  call void @_Py_IncRef(ptr noundef nonnull %_2.i.i.i.i.i.i) #ATTR, !noalias !ID
; invoke <pyo3::err::PyErr>::get_type
  %285 = invoke noundef nonnull ptr @<pyo3::err::PyErr>::get_type(ptr noundef nonnull align 8 dereferenceable(48) %_3.i1.i)
          to label %bb1.i.i8.i unwind label %bb4.i.i.i6.i, !noalias !ID

bb4.i.i.i6.i:                                     ; preds = %bb1.i5.i
  %286 = landingpad { ptr, i32 }
          cleanup
  call void @_Py_DecRef(ptr noundef nonnull %_2.i.i.i.i.i.i) #ATTR, !noalias !ID
  br label %bb12.i.i.i

cleanup.i.i.i:                                    ; preds = %bb2.i.i9.i
  %287 = landingpad { ptr, i32 }
          cleanup
  br label %bb12.i.i.i

bb1.i.i8.i:                                       ; preds = %bb1.i5.i
  %_8.i.i.i.i = call noundef i32 @PyErr_GivenExceptionMatches(ptr noundef nonnull %285, ptr noundef nonnull %_2.i.i.i.i.i.i) #ATTR, !noalias !ID
  call void @_Py_DecRef(ptr noundef nonnull %285) #ATTR, !noalias !ID
  %_0.i.not.i.i.i = icmp eq i32 %_8.i.i.i.i, 0
  call void @_Py_DecRef(ptr noundef nonnull %_2.i.i.i.i.i.i) #ATTR, !noalias !ID
  br i1 %_0.i.not.i.i.i, label %bb3.i.i31.i, label %bb2.i.i9.i

bb3.i.i31.i:                                      ; preds = %bb1.i.i8.i
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %_4.sroa.0.i, ptr noundef nonnull align 8 dereferenceable(16) %_3.i1.i, i64 16, i1 false), !alias.scope !ID, !noalias !ID
  %_4.sroa.6.0._3.i1.sroa_idx.i = getelementptr inbounds nuw i8, ptr %_3.i1.i, i64 16
  %_4.sroa.6.0.copyload34.i = load i64, ptr %_4.sroa.6.0._3.i1.sroa_idx.i, align 8, !alias.scope !ID, !noalias !ID
  %_4.sroa.7.0._3.i1.sroa_idx.i = getelementptr inbounds nuw i8, ptr %_3.i1.i, i64 24
  %288 = load <2 x ptr>, ptr %_4.sroa.7.0._3.i1.sroa_idx.i, align 8, !alias.scope !ID, !noalias !ID
  %_4.sroa.9.0._3.i1.sroa_idx.i = getelementptr inbounds nuw i8, ptr %_3.i1.i, i64 40
  %_4.sroa.9.0.copyload43.i = load i64, ptr %_4.sroa.9.0._3.i1.sroa_idx.i, align 8, !alias.scope !ID, !noalias !ID
  br label %<pyo3::marker::Python>::attach::<purrdf_native::py_store::presentation::settled<pyo3::instance::Py<purrdf_native::py_store::query::PyQueryOutcome>, <purrdf_native::py_store::quad_store::PyQuadStore>::query_governed::{closure#0}>::{closure#0}::{closure#0}, pyo3::err::PyErr> (.exit.i)

bb2.i.i9.i:                                       ; preds = %bb1.i.i8.i
; invoke <pyo3::err::PyErr>::value
  %value.i.i.i = invoke noundef nonnull align 8 ptr @<pyo3::err::PyErr>::value(ptr noundef nonnull align 8 dereferenceable(48) %_3.i1.i)
          to label %bb2.lr.ph.i.i.i.i unwind label %cleanup.i.i.i, !noalias !ID

bb2.lr.ph.i.i.i.i:                                ; preds = %bb2.i.i9.i
  %289 = getelementptr inbounds nuw i8, ptr %_4.i.i.i.i.i.i.i, i64 1
  %290 = getelementptr inbounds nuw i8, ptr %_4.i.i.i.i.i.i.i, i64 8
  %291 = getelementptr inbounds nuw i8, ptr %_9.i.i.i.i, i64 8
  call void @llvm.lifetime.start.p0(ptr nonnull %_9.i.i.i.i), !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %_4.i.i.i.i.i.i.i), !noalias !ID
; invoke <pyo3::types::string::PyString>::new
  %_3.i.i.i.i.i.i4.i.i.i = invoke noundef nonnull ptr @<pyo3::types::string::PyString>::new(ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @alloc_016e71ba2f68cc7172262ae988bb360c, i64 noundef 10)
          to label %_3.i.i.i.i.i.i.noexc.i.i.i unwind label %cleanup1.i.i.i, !noalias !ID

_3.i.i.i.i.i.i.noexc.i.i.i:                       ; preds = %bb2.lr.ph.i.i.i.i
; invoke <pyo3::instance::Bound<pyo3::types::any::PyAny> as pyo3::types::any::PyAnyMethods>::hasattr::inner
  invoke void @<pyo3::instance::Bound<pyo3::types::any::PyAny> as pyo3::types::any::PyAnyMethods>::hasattr::inner(ptr noalias nofree noundef nonnull sret([56 x i8]) align 8 captures(none) dereferenceable(56) %_4.i.i.i.i.i.i.i, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(8) %value.i.i.i, ptr noundef nonnull %_3.i.i.i.i.i.i4.i.i.i)
          to label %<pyo3::instance::Bound<pyo3::types::any::PyAny> as pyo3::types::any::PyAnyMethods>::hasattr::<&str> (.exit.i.i.i.i.i.i.i) unwind label %bb8.i.i.i.i.i.i.i.i, !noalias !ID

bb8.i.i.i.i.i.i.i.i:                              ; preds = %_3.i.i.i.i.i.i.noexc.1.i.i.i, %_3.i.i.i.i.i.i.noexc.i.i.i
  %_3.i.i.i.i.i.i4.lcssa.i.i.i = phi ptr [ %_3.i.i.i.i.i.i4.i.i.i, %_3.i.i.i.i.i.i.noexc.i.i.i ], [ %_3.i.i.i.i.i.i4.1.i.i.i, %_3.i.i.i.i.i.i.noexc.1.i.i.i ]
  %292 = landingpad { ptr, i32 }
          cleanup
  call void @_Py_DecRef(ptr noundef nonnull %_3.i.i.i.i.i.i4.lcssa.i.i.i) #ATTR, !noalias !ID
  br label %bb12.i.i.i

<pyo3::instance::Bound<pyo3::types::any::PyAny> as pyo3::types::any::PyAnyMethods>::hasattr::<&str> (.exit.i.i.i.i.i.i.i): ; preds = %_3.i.i.i.i.i.i.noexc.i.i.i
  call void @_Py_DecRef(ptr noundef nonnull %_3.i.i.i.i.i.i4.i.i.i) #ATTR, !noalias !ID
  %293 = load i8, ptr %_4.i.i.i.i.i.i.i, align 8, !range !ID, !noalias !ID, !noundef !ID
  %294 = trunc nuw i8 %293 to i1
  br i1 %294, label %<core::array::iter::iter_inner::PolymorphicIter<[core::mem::maybe_uninit::MaybeUninit<&str>]>>::try_fold::<(), core::iter::traits::iterator::Iterator::try_for_each::call<&str, core::result::Result<(), pyo3::err::PyErr>, purrdf_native::py_store::presentation::settled<pyo3::instance::Py<purrdf_native::py_store::query::PyQueryOutcome>, <purrdf_native::py_store::quad_store::PyQuadStore>::query_governed::{closure#0}>::{closure#0}::{closure#0}::{closure#0}>::{closure#0}, core::result::Result<(), pyo3::err::PyErr>>::{closure#0} (.exit.thread.i.i.i.i), label %bb8.i.i.i.i.i.i10.i

<core::array::iter::iter_inner::PolymorphicIter<[core::mem::maybe_uninit::MaybeUninit<&str>]>>::try_fold::<(), core::iter::traits::iterator::Iterator::try_for_each::call<&str, core::result::Result<(), pyo3::err::PyErr>, purrdf_native::py_store::presentation::settled<pyo3::instance::Py<purrdf_native::py_store::query::PyQueryOutcome>, <purrdf_native::py_store::quad_store::PyQuadStore>::query_governed::{closure#0}>::{closure#0}::{closure#0}::{closure#0}>::{closure#0}, core::result::Result<(), pyo3::err::PyErr>>::{closure#0} (.exit.thread.i.i.i.i): ; preds = %<pyo3::instance::Bound<pyo3::types::any::PyAny> as pyo3::types::any::PyAnyMethods>::hasattr::<&str> (.exit.i.i.i.i.1.i.i.i), %<pyo3::instance::Bound<pyo3::types::any::PyAny> as pyo3::types::any::PyAnyMethods>::hasattr::<&str> (.exit.i.i.i.i.i.i.i)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %291, ptr noundef nonnull align 8 dereferenceable(48) %290, i64 48, i1 false), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_4.i.i.i.i.i.i.i), !noalias !ID
  br label %bb5.i.i.i.i

bb8.i.i.i.i.i.i10.i:                              ; preds = %<pyo3::instance::Bound<pyo3::types::any::PyAny> as pyo3::types::any::PyAnyMethods>::hasattr::<&str> (.exit.i.i.i.i.i.i.i)
  %295 = load i8, ptr %289, align 1, !range !ID, !noalias !ID, !noundef !ID
  %_16.i.i.i.i.i.i.i = trunc nuw i8 %295 to i1
  br i1 %_16.i.i.i.i.i.i.i, label %<core::array::iter::iter_inner::PolymorphicIter<[core::mem::maybe_uninit::MaybeUninit<&str>]>>::try_fold::<(), core::iter::traits::iterator::Iterator::try_for_each::call<&str, core::result::Result<(), pyo3::err::PyErr>, purrdf_native::py_store::presentation::settled<pyo3::instance::Py<purrdf_native::py_store::query::PyQueryOutcome>, <purrdf_native::py_store::quad_store::PyQuadStore>::query_governed::{closure#0}>::{closure#0}::{closure#0}::{closure#0}>::{closure#0}, core::result::Result<(), pyo3::err::PyErr>>::{closure#0} (.exit.thread11.i.i.i.i), label %<core::array::iter::iter_inner::PolymorphicIter<[core::mem::maybe_uninit::MaybeUninit<&str>]>>::try_fold::<(), core::iter::traits::iterator::Iterator::try_for_each::call<&str, core::result::Result<(), pyo3::err::PyErr>, purrdf_native::py_store::presentation::settled<pyo3::instance::Py<purrdf_native::py_store::query::PyQueryOutcome>, <purrdf_native::py_store::quad_store::PyQuadStore>::query_governed::{closure#0}>::{closure#0}::{closure#0}::{closure#0}>::{closure#0}, core::result::Result<(), pyo3::err::PyErr>>::{closure#0} (.exit.i.i.i.i)

<core::array::iter::iter_inner::PolymorphicIter<[core::mem::maybe_uninit::MaybeUninit<&str>]>>::try_fold::<(), core::iter::traits::iterator::Iterator::try_for_each::call<&str, core::result::Result<(), pyo3::err::PyErr>, purrdf_native::py_store::presentation::settled<pyo3::instance::Py<purrdf_native::py_store::query::PyQueryOutcome>, <purrdf_native::py_store::quad_store::PyQuadStore>::query_governed::{closure#0}>::{closure#0}::{closure#0}::{closure#0}>::{closure#0}, core::result::Result<(), pyo3::err::PyErr>>::{closure#0} (.exit.thread11.i.i.i.i): ; preds = %bb8.i.i.i.i.i.i10.i
  call void @llvm.lifetime.end.p0(ptr nonnull %_4.i.i.i.i.i.i.i), !noalias !ID
  br label %bb6.i.i.i11.i

<core::array::iter::iter_inner::PolymorphicIter<[core::mem::maybe_uninit::MaybeUninit<&str>]>>::try_fold::<(), core::iter::traits::iterator::Iterator::try_for_each::call<&str, core::result::Result<(), pyo3::err::PyErr>, purrdf_native::py_store::presentation::settled<pyo3::instance::Py<purrdf_native::py_store::query::PyQueryOutcome>, <purrdf_native::py_store::quad_store::PyQuadStore>::query_governed::{closure#0}>::{closure#0}::{closure#0}::{closure#0}>::{closure#0}, core::result::Result<(), pyo3::err::PyErr>>::{closure#0} (.exit.i.i.i.i): ; preds = %bb8.i.i.i.i.i.i10.i
  %_26.i.i.i.i.i.i.i = call noundef ptr @Py_GetConstantBorrowed(i32 noundef 0) #ATTR, !noalias !ID
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %_26.i.i.i.i.i.i.i) ]
  call void @_Py_IncRef(ptr noundef nonnull %_26.i.i.i.i.i.i.i) #ATTR, !noalias !ID
; invoke <pyo3::instance::Bound<pyo3::types::any::PyAny> as pyo3::types::any::PyAnyMethods>::setattr::<&str, pyo3::instance::Py<pyo3::types::any::PyAny>>
  invoke fastcc void @<pyo3::instance::Bound<pyo3::types::any::PyAny> as pyo3::types::any::PyAnyMethods>::setattr::<&str, pyo3::instance::Py<pyo3::types::any::PyAny>>(ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(56) %_9.i.i.i.i, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(8) %value.i.i.i, ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @alloc_016e71ba2f68cc7172262ae988bb360c, i64 noundef 10, ptr noundef nonnull %_26.i.i.i.i.i.i.i)
          to label %.noexc.i.i.i unwind label %cleanup1.i.i.i, !noalias !ID

.noexc.i.i.i:                                     ; preds = %<core::array::iter::iter_inner::PolymorphicIter<[core::mem::maybe_uninit::MaybeUninit<&str>]>>::try_fold::<(), core::iter::traits::iterator::Iterator::try_for_each::call<&str, core::result::Result<(), pyo3::err::PyErr>, purrdf_native::py_store::presentation::settled<pyo3::instance::Py<purrdf_native::py_store::query::PyQueryOutcome>, <purrdf_native::py_store::quad_store::PyQuadStore>::query_governed::{closure#0}>::{closure#0}::{closure#0}::{closure#0}>::{closure#0}, core::result::Result<(), pyo3::err::PyErr>>::{closure#0} (.exit.i.i.i.i)
  %_2.i.pre.i.i.i.i = load i64, ptr %_9.i.i.i.i, align 8, !range !ID, !alias.scope !ID, !noalias !ID
  %296 = trunc nuw i64 %_2.i.pre.i.i.i.i to i1
  call void @llvm.lifetime.end.p0(ptr nonnull %_4.i.i.i.i.i.i.i), !noalias !ID
  br i1 %296, label %bb5.i.i.i.i, label %bb6.i.i.i11.i

bb6.i.i.i11.i:                                    ; preds = %.noexc.i.i.i, %<core::array::iter::iter_inner::PolymorphicIter<[core::mem::maybe_uninit::MaybeUninit<&str>]>>::try_fold::<(), core::iter::traits::iterator::Iterator::try_for_each::call<&str, core::result::Result<(), pyo3::err::PyErr>, purrdf_native::py_store::presentation::settled<pyo3::instance::Py<purrdf_native::py_store::query::PyQueryOutcome>, <purrdf_native::py_store::quad_store::PyQuadStore>::query_governed::{closure#0}>::{closure#0}::{closure#0}::{closure#0}>::{closure#0}, core::result::Result<(), pyo3::err::PyErr>>::{closure#0} (.exit.thread11.i.i.i.i)
  call void @llvm.lifetime.end.p0(ptr nonnull %_9.i.i.i.i), !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %_9.i.i.i.i), !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %_4.i.i.i.i.i.i.i), !noalias !ID
; invoke <pyo3::types::string::PyString>::new
  %_3.i.i.i.i.i.i4.1.i.i.i = invoke noundef nonnull ptr @<pyo3::types::string::PyString>::new(ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @alloc_6f6b49b405cc516c15819f32fccb7974, i64 noundef 12)
          to label %_3.i.i.i.i.i.i.noexc.1.i.i.i unwind label %cleanup1.i.i.i, !noalias !ID

_3.i.i.i.i.i.i.noexc.1.i.i.i:                     ; preds = %bb6.i.i.i11.i
; invoke <pyo3::instance::Bound<pyo3::types::any::PyAny> as pyo3::types::any::PyAnyMethods>::hasattr::inner
  invoke void @<pyo3::instance::Bound<pyo3::types::any::PyAny> as pyo3::types::any::PyAnyMethods>::hasattr::inner(ptr noalias nofree noundef nonnull sret([56 x i8]) align 8 captures(none) dereferenceable(56) %_4.i.i.i.i.i.i.i, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(8) %value.i.i.i, ptr noundef nonnull %_3.i.i.i.i.i.i4.1.i.i.i)
          to label %<pyo3::instance::Bound<pyo3::types::any::PyAny> as pyo3::types::any::PyAnyMethods>::hasattr::<&str> (.exit.i.i.i.i.1.i.i.i) unwind label %bb8.i.i.i.i.i.i.i.i, !noalias !ID

<pyo3::instance::Bound<pyo3::types::any::PyAny> as pyo3::types::any::PyAnyMethods>::hasattr::<&str> (.exit.i.i.i.i.1.i.i.i): ; preds = %_3.i.i.i.i.i.i.noexc.1.i.i.i
  call void @_Py_DecRef(ptr noundef nonnull %_3.i.i.i.i.i.i4.1.i.i.i) #ATTR, !noalias !ID
  %297 = load i8, ptr %_4.i.i.i.i.i.i.i, align 8, !range !ID, !noalias !ID, !noundef !ID
  %298 = trunc nuw i8 %297 to i1
  br i1 %298, label %<core::array::iter::iter_inner::PolymorphicIter<[core::mem::maybe_uninit::MaybeUninit<&str>]>>::try_fold::<(), core::iter::traits::iterator::Iterator::try_for_each::call<&str, core::result::Result<(), pyo3::err::PyErr>, purrdf_native::py_store::presentation::settled<pyo3::instance::Py<purrdf_native::py_store::query::PyQueryOutcome>, <purrdf_native::py_store::quad_store::PyQuadStore>::query_governed::{closure#0}>::{closure#0}::{closure#0}::{closure#0}>::{closure#0}, core::result::Result<(), pyo3::err::PyErr>>::{closure#0} (.exit.thread.i.i.i.i), label %bb8.i.i.i.i.1.i.i.i

bb8.i.i.i.i.1.i.i.i:                              ; preds = %<pyo3::instance::Bound<pyo3::types::any::PyAny> as pyo3::types::any::PyAnyMethods>::hasattr::<&str> (.exit.i.i.i.i.1.i.i.i)
  %299 = load i8, ptr %289, align 1, !range !ID, !noalias !ID, !noundef !ID
  %_16.i.i.i.i.1.i.i.i = trunc nuw i8 %299 to i1
  br i1 %_16.i.i.i.i.1.i.i.i, label %<core::array::iter::iter_inner::PolymorphicIter<[core::mem::maybe_uninit::MaybeUninit<&str>]>>::try_fold::<(), core::iter::traits::iterator::Iterator::try_for_each::call<&str, core::result::Result<(), pyo3::err::PyErr>, purrdf_native::py_store::presentation::settled<pyo3::instance::Py<purrdf_native::py_store::query::PyQueryOutcome>, <purrdf_native::py_store::quad_store::PyQuadStore>::query_governed::{closure#0}>::{closure#0}::{closure#0}::{closure#0}>::{closure#0}, core::result::Result<(), pyo3::err::PyErr>>::{closure#0} (.exit.thread11.i.1.i.i.i), label %<core::array::iter::iter_inner::PolymorphicIter<[core::mem::maybe_uninit::MaybeUninit<&str>]>>::try_fold::<(), core::iter::traits::iterator::Iterator::try_for_each::call<&str, core::result::Result<(), pyo3::err::PyErr>, purrdf_native::py_store::presentation::settled<pyo3::instance::Py<purrdf_native::py_store::query::PyQueryOutcome>, <purrdf_native::py_store::quad_store::PyQuadStore>::query_governed::{closure#0}>::{closure#0}::{closure#0}::{closure#0}>::{closure#0}, core::result::Result<(), pyo3::err::PyErr>>::{closure#0} (.exit.i.1.i.i.i)

<core::array::iter::iter_inner::PolymorphicIter<[core::mem::maybe_uninit::MaybeUninit<&str>]>>::try_fold::<(), core::iter::traits::iterator::Iterator::try_for_each::call<&str, core::result::Result<(), pyo3::err::PyErr>, purrdf_native::py_store::presentation::settled<pyo3::instance::Py<purrdf_native::py_store::query::PyQueryOutcome>, <purrdf_native::py_store::quad_store::PyQuadStore>::query_governed::{closure#0}>::{closure#0}::{closure#0}::{closure#0}>::{closure#0}, core::result::Result<(), pyo3::err::PyErr>>::{closure#0} (.exit.i.1.i.i.i): ; preds = %bb8.i.i.i.i.1.i.i.i
  %_26.i.i.i.i.1.i.i.i = call noundef ptr @Py_GetConstantBorrowed(i32 noundef 0) #ATTR, !noalias !ID
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %_26.i.i.i.i.1.i.i.i) ]
  call void @_Py_IncRef(ptr noundef nonnull %_26.i.i.i.i.1.i.i.i) #ATTR, !noalias !ID
; invoke <pyo3::instance::Bound<pyo3::types::any::PyAny> as pyo3::types::any::PyAnyMethods>::setattr::<&str, pyo3::instance::Py<pyo3::types::any::PyAny>>
  invoke fastcc void @<pyo3::instance::Bound<pyo3::types::any::PyAny> as pyo3::types::any::PyAnyMethods>::setattr::<&str, pyo3::instance::Py<pyo3::types::any::PyAny>>(ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(56) %_9.i.i.i.i, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(8) %value.i.i.i, ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @alloc_6f6b49b405cc516c15819f32fccb7974, i64 noundef 12, ptr noundef nonnull %_26.i.i.i.i.1.i.i.i)
          to label %.noexc.1.i.i.i unwind label %cleanup1.i.i.i, !noalias !ID

.noexc.1.i.i.i:                                   ; preds = %<core::array::iter::iter_inner::PolymorphicIter<[core::mem::maybe_uninit::MaybeUninit<&str>]>>::try_fold::<(), core::iter::traits::iterator::Iterator::try_for_each::call<&str, core::result::Result<(), pyo3::err::PyErr>, purrdf_native::py_store::presentation::settled<pyo3::instance::Py<purrdf_native::py_store::query::PyQueryOutcome>, <purrdf_native::py_store::quad_store::PyQuadStore>::query_governed::{closure#0}>::{closure#0}::{closure#0}::{closure#0}>::{closure#0}, core::result::Result<(), pyo3::err::PyErr>>::{closure#0} (.exit.i.1.i.i.i)
  %_2.i.pre.i.1.i.i.i = load i64, ptr %_9.i.i.i.i, align 8, !range !ID, !alias.scope !ID, !noalias !ID
  %300 = trunc nuw i64 %_2.i.pre.i.1.i.i.i to i1
  call void @llvm.lifetime.end.p0(ptr nonnull %_4.i.i.i.i.i.i.i), !noalias !ID
  br i1 %300, label %bb5.i.i.i.i, label %bb4.i6.i.i.i

<core::array::iter::iter_inner::PolymorphicIter<[core::mem::maybe_uninit::MaybeUninit<&str>]>>::try_fold::<(), core::iter::traits::iterator::Iterator::try_for_each::call<&str, core::result::Result<(), pyo3::err::PyErr>, purrdf_native::py_store::presentation::settled<pyo3::instance::Py<purrdf_native::py_store::query::PyQueryOutcome>, <purrdf_native::py_store::quad_store::PyQuadStore>::query_governed::{closure#0}>::{closure#0}::{closure#0}::{closure#0}>::{closure#0}, core::result::Result<(), pyo3::err::PyErr>>::{closure#0} (.exit.thread11.i.1.i.i.i): ; preds = %bb8.i.i.i.i.1.i.i.i
  call void @llvm.lifetime.end.p0(ptr nonnull %_4.i.i.i.i.i.i.i), !noalias !ID
  br label %bb4.i6.i.i.i

cleanup1.i.i.i:                                   ; preds = %<core::array::iter::iter_inner::PolymorphicIter<[core::mem::maybe_uninit::MaybeUninit<&str>]>>::try_fold::<(), core::iter::traits::iterator::Iterator::try_for_each::call<&str, core::result::Result<(), pyo3::err::PyErr>, purrdf_native::py_store::presentation::settled<pyo3::instance::Py<purrdf_native::py_store::query::PyQueryOutcome>, <purrdf_native::py_store::quad_store::PyQuadStore>::query_governed::{closure#0}>::{closure#0}::{closure#0}::{closure#0}>::{closure#0}, core::result::Result<(), pyo3::err::PyErr>>::{closure#0} (.exit.i.1.i.i.i), %bb6.i.i.i11.i, %<core::array::iter::iter_inner::PolymorphicIter<[core::mem::maybe_uninit::MaybeUninit<&str>]>>::try_fold::<(), core::iter::traits::iterator::Iterator::try_for_each::call<&str, core::result::Result<(), pyo3::err::PyErr>, purrdf_native::py_store::presentation::settled<pyo3::instance::Py<purrdf_native::py_store::query::PyQueryOutcome>, <purrdf_native::py_store::quad_store::PyQuadStore>::query_governed::{closure#0}>::{closure#0}::{closure#0}::{closure#0}>::{closure#0}, core::result::Result<(), pyo3::err::PyErr>>::{closure#0} (.exit.i.i.i.i), %bb2.lr.ph.i.i.i.i
  %301 = landingpad { ptr, i32 }
          cleanup
  br label %bb12.i.i.i

bb4.i6.i.i.i:                                     ; preds = %<core::array::iter::iter_inner::PolymorphicIter<[core::mem::maybe_uninit::MaybeUninit<&str>]>>::try_fold::<(), core::iter::traits::iterator::Iterator::try_for_each::call<&str, core::result::Result<(), pyo3::err::PyErr>, purrdf_native::py_store::presentation::settled<pyo3::instance::Py<purrdf_native::py_store::query::PyQueryOutcome>, <purrdf_native::py_store::quad_store::PyQuadStore>::query_governed::{closure#0}>::{closure#0}::{closure#0}::{closure#0}>::{closure#0}, core::result::Result<(), pyo3::err::PyErr>>::{closure#0} (.exit.thread11.i.1.i.i.i), %.noexc.1.i.i.i
  call void @llvm.lifetime.end.p0(ptr nonnull %_9.i.i.i.i), !noalias !ID
  %_13.sroa.4.0._1.sroa_idx.i7.i.i = getelementptr inbounds nuw i8, ptr %_3.i1.i, i64 16
  %_13.sroa.4.0.copyload.i8.i.i = load i64, ptr %_13.sroa.4.0._1.sroa_idx.i7.i.i, align 8, !alias.scope !ID, !noalias !ID
  %_13.sroa.5.0._1.sroa_idx.i9.i.i = getelementptr inbounds nuw i8, ptr %_3.i1.i, i64 24
  %302 = load <2 x ptr>, ptr %_13.sroa.5.0._1.sroa_idx.i9.i.i, align 8, !alias.scope !ID, !noalias !ID
  %_13.sroa.7.0._1.sroa_idx.i.i.i = getelementptr inbounds nuw i8, ptr %_3.i1.i, i64 40
  %_13.sroa.7.0.copyload.i.i.i = load i64, ptr %_13.sroa.7.0._1.sroa_idx.i.i.i, align 8, !alias.scope !ID, !noalias !ID
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %_4.sroa.0.i, ptr noundef nonnull align 8 dereferenceable(16) %_3.i1.i, i64 16, i1 false), !alias.scope !ID, !noalias !ID
  br label %<pyo3::marker::Python>::attach::<purrdf_native::py_store::presentation::settled<pyo3::instance::Py<purrdf_native::py_store::query::PyQueryOutcome>, <purrdf_native::py_store::quad_store::PyQuadStore>::query_governed::{closure#0}>::{closure#0}::{closure#0}, pyo3::err::PyErr> (.exit.i)

bb5.i.i.i.i:                                      ; preds = %.noexc.1.i.i.i, %.noexc.i.i.i, %<core::array::iter::iter_inner::PolymorphicIter<[core::mem::maybe_uninit::MaybeUninit<&str>]>>::try_fold::<(), core::iter::traits::iterator::Iterator::try_for_each::call<&str, core::result::Result<(), pyo3::err::PyErr>, purrdf_native::py_store::presentation::settled<pyo3::instance::Py<purrdf_native::py_store::query::PyQueryOutcome>, <purrdf_native::py_store::quad_store::PyQuadStore>::query_governed::{closure#0}>::{closure#0}::{closure#0}::{closure#0}>::{closure#0}, core::result::Result<(), pyo3::err::PyErr>>::{closure#0} (.exit.thread.i.i.i.i)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %_4.sroa.0.i, ptr noundef nonnull align 8 dereferenceable(16) %291, i64 16, i1 false), !noalias !ID
  %_4.sroa.6.0..sroa_idx32.i = getelementptr inbounds nuw i8, ptr %_9.i.i.i.i, i64 24
  %_4.sroa.6.0.copyload33.i = load i64, ptr %_4.sroa.6.0..sroa_idx32.i, align 8, !noalias !ID
  %_4.sroa.7.0..sroa_idx35.i = getelementptr inbounds nuw i8, ptr %_9.i.i.i.i, i64 32
  %303 = load <2 x ptr>, ptr %_4.sroa.7.0..sroa_idx35.i, align 8, !noalias !ID
  %_4.sroa.9.0..sroa_idx41.i = getelementptr inbounds nuw i8, ptr %_9.i.i.i.i, i64 48
  %_4.sroa.9.0.copyload42.i = load i64, ptr %_4.sroa.9.0..sroa_idx41.i, align 8, !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_9.i.i.i.i), !noalias !ID
  %_13.sroa.4.0._1.sroa_idx.i.i.i = getelementptr inbounds nuw i8, ptr %_3.i1.i, i64 16
  %_13.sroa.4.0.copyload.i.i.i = load i64, ptr %_13.sroa.4.0._1.sroa_idx.i.i.i, align 8, !alias.scope !ID, !noalias !ID
  %_13.sroa.5.0._1.sroa_idx.i.i.i = getelementptr inbounds nuw i8, ptr %_3.i1.i, i64 24
  %_13.sroa.5.0.copyload.i.i.i = load ptr, ptr %_13.sroa.5.0._1.sroa_idx.i.i.i, align 8, !alias.scope !ID, !noalias !ID
  %_13.sroa.6.0._1.sroa_idx.i.i.i = getelementptr inbounds nuw i8, ptr %_3.i1.i, i64 32
  %_13.sroa.6.0.copyload.i.i.i = load ptr, ptr %_13.sroa.6.0._1.sroa_idx.i.i.i, align 8, !alias.scope !ID, !noalias !ID
  %304 = icmp eq i64 %_13.sroa.4.0.copyload.i.i.i, 0
  br i1 %304, label %<pyo3::marker::Python>::attach::<purrdf_native::py_store::presentation::settled<pyo3::instance::Py<purrdf_native::py_store::query::PyQueryOutcome>, <purrdf_native::py_store::quad_store::PyQuadStore>::query_governed::{closure#0}>::{closure#0}::{closure#0}, pyo3::err::PyErr> (.exit.i), label %bb2.i.i.i.i.i.i12.i

bb2.i.i.i.i.i.i12.i:                              ; preds = %bb5.i.i.i.i
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %_13.sroa.6.0.copyload.i.i.i) ]
  %.not.i.i.i.i.i.i.i13.i = icmp eq ptr %_13.sroa.5.0.copyload.i.i.i, null
  br i1 %.not.i.i.i.i.i.i.i13.i, label %bb3.i.i.i.i.i.i.i25.i, label %bb2.i.i.i.i.i.i.i14.i

bb2.i.i.i.i.i.i.i14.i:                            ; preds = %bb2.i.i.i.i.i.i12.i
  %305 = load ptr, ptr %_13.sroa.6.0.copyload.i.i.i, align 8, !invariant.load !ID, !noalias !ID
  %.not.i.i.i.i.i.i.i.i15.i = icmp eq ptr %305, null
  br i1 %.not.i.i.i.i.i.i.i.i15.i, label %bb3.i.i.i.i.i.i.i.i21.i, label %is_not_null.i.i.i.i.i.i.i.i16.i

is_not_null.i.i.i.i.i.i.i.i16.i:                  ; preds = %bb2.i.i.i.i.i.i.i14.i
  invoke void %305(ptr noundef nonnull %_13.sroa.5.0.copyload.i.i.i)
          to label %bb3.i.i.i.i.i.i.i.i21.i unwind label %cleanup.i.i.i.i.i.i.i.i17.i, !noalias !ID

bb3.i.i.i.i.i.i.i.i21.i:                          ; preds = %is_not_null.i.i.i.i.i.i.i.i16.i, %bb2.i.i.i.i.i.i.i14.i
  %306 = getelementptr inbounds nuw i8, ptr %_13.sroa.6.0.copyload.i.i.i, i64 8
  %size.i.i.i.i.i.i.i.i.i22.i = load i64, ptr %306, align 8, !range !ID, !invariant.load !ID, !noalias !ID
  %307 = icmp eq i64 %size.i.i.i.i.i.i.i.i.i22.i, 0
  br i1 %307, label %<pyo3::marker::Python>::attach::<purrdf_native::py_store::presentation::settled<pyo3::instance::Py<purrdf_native::py_store::query::PyQueryOutcome>, <purrdf_native::py_store::quad_store::PyQuadStore>::query_governed::{closure#0}>::{closure#0}::{closure#0}, pyo3::err::PyErr> (.exit.i), label %<alloc::alloc::Global as core::alloc::Allocator>::deallocate (.exit.i.i.i.i.i.i.i.i.i23.i)

<alloc::alloc::Global as core::alloc::Allocator>::deallocate (.exit.i.i.i.i.i.i.i.i.i23.i): ; preds = %bb3.i.i.i.i.i.i.i.i21.i
  %308 = getelementptr inbounds nuw i8, ptr %_13.sroa.6.0.copyload.i.i.i, i64 16
  %align.i.i.i.i.i.i.i.i.i24.i = load i64, ptr %308, align 8, !range !ID, !invariant.load !ID, !noalias !ID
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %_13.sroa.5.0.copyload.i.i.i, i64 noundef %size.i.i.i.i.i.i.i.i.i22.i, i64 noundef range(i64 1, -9223372036854775807) %align.i.i.i.i.i.i.i.i.i24.i) #ATTR, !noalias !ID
  br label %<pyo3::marker::Python>::attach::<purrdf_native::py_store::presentation::settled<pyo3::instance::Py<purrdf_native::py_store::query::PyQueryOutcome>, <purrdf_native::py_store::quad_store::PyQuadStore>::query_governed::{closure#0}>::{closure#0}::{closure#0}, pyo3::err::PyErr> (.exit.i)

cleanup.i.i.i.i.i.i.i.i17.i:                      ; preds = %is_not_null.i.i.i.i.i.i.i.i16.i
  %309 = landingpad { ptr, i32 }
          cleanup
  %310 = getelementptr inbounds nuw i8, ptr %_13.sroa.6.0.copyload.i.i.i, i64 8
  %size.i4.i.i.i.i.i.i.i.i18.i = load i64, ptr %310, align 8, !range !ID, !invariant.load !ID, !noalias !ID
  %311 = icmp eq i64 %size.i4.i.i.i.i.i.i.i.i18.i, 0
  br i1 %311, label %cleanup1.body.i.i, label %<alloc::alloc::Global as core::alloc::Allocator>::deallocate (.exit.i5.i.i.i.i.i.i.i.i19.i)

<alloc::alloc::Global as core::alloc::Allocator>::deallocate (.exit.i5.i.i.i.i.i.i.i.i19.i): ; preds = %cleanup.i.i.i.i.i.i.i.i17.i
  %312 = getelementptr inbounds nuw i8, ptr %_13.sroa.6.0.copyload.i.i.i, i64 16
  %align.i6.i.i.i.i.i.i.i.i20.i = load i64, ptr %312, align 8, !range !ID, !invariant.load !ID, !noalias !ID
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %_13.sroa.5.0.copyload.i.i.i, i64 noundef %size.i4.i.i.i.i.i.i.i.i18.i, i64 noundef range(i64 1, -9223372036854775807) %align.i6.i.i.i.i.i.i.i.i20.i) #ATTR, !noalias !ID
  br label %cleanup1.body.i.i

bb3.i.i.i.i.i.i.i25.i:                            ; preds = %bb2.i.i.i.i.i.i12.i
  %_3.i.i.i.i.i.i.i.i.i.i.i.i.i.i26.i = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @pyo3::internal::state::ATTACH_COUNT::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL)
  %self3.val.i.i.i.i.i.i.i.i.i.i.i.i27.i = load i64, ptr %_3.i.i.i.i.i.i.i.i.i.i.i.i.i.i26.i, align 8, !noalias !ID, !noundef !ID
  %_0.i.i.i.i.i.i.i.i.i.i.i.i.i28.i = icmp sgt i64 %self3.val.i.i.i.i.i.i.i.i.i.i.i.i27.i, 0
  br i1 %_0.i.i.i.i.i.i.i.i.i.i.i.i.i28.i, label %bb1.i.i.i.i.i.i.i.i.i.i.i30.i, label %bb2.i.i.i.i.i.i.i.i.i.i.i29.i, !prof !ID

bb2.i.i.i.i.i.i.i.i.i.i.i29.i:                    ; preds = %bb3.i.i.i.i.i.i.i25.i
; invoke <pyo3::instance::Py<_> as core::ops::drop::Drop>::drop::drop_slow
  invoke void @<pyo3::instance::Py<_> as core::ops::drop::Drop>::drop::drop_slow(ptr noundef nonnull %_13.sroa.6.0.copyload.i.i.i)
          to label %<pyo3::marker::Python>::attach::<purrdf_native::py_store::presentation::settled<pyo3::instance::Py<purrdf_native::py_store::query::PyQueryOutcome>, <purrdf_native::py_store::quad_store::PyQuadStore>::query_governed::{closure#0}>::{closure#0}::{closure#0}, pyo3::err::PyErr> (.exit.i) unwind label %cleanup1.i.i, !noalias !ID

bb1.i.i.i.i.i.i.i.i.i.i.i30.i:                    ; preds = %bb3.i.i.i.i.i.i.i25.i
  call void @_Py_DecRef(ptr noundef nonnull %_13.sroa.6.0.copyload.i.i.i) #ATTR, !noalias !ID
  br label %<pyo3::marker::Python>::attach::<purrdf_native::py_store::presentation::settled<pyo3::instance::Py<purrdf_native::py_store::query::PyQueryOutcome>, <purrdf_native::py_store::quad_store::PyQuadStore>::query_governed::{closure#0}>::{closure#0}::{closure#0}, pyo3::err::PyErr> (.exit.i)

terminate.i.i7.i:                                 ; preds = %bb12.i.i.i
  %313 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #ATTR, !noalias !ID
  unreachable

bb12.i.i.i:                                       ; preds = %cleanup1.i.i.i, %bb8.i.i.i.i.i.i.i.i, %cleanup.i.i.i, %bb4.i.i.i6.i
  %.pn.ph.i.i.i = phi { ptr, i32 } [ %286, %bb4.i.i.i6.i ], [ %287, %cleanup.i.i.i ], [ %301, %cleanup1.i.i.i ], [ %292, %bb8.i.i.i.i.i.i.i.i ]
; invoke core::ptr::drop_glue::<pyo3::err::PyErr>
  invoke void @core::ptr::drop_glue::<pyo3::err::PyErr>(ptr noalias nofree noundef nonnull align 8 dereferenceable(48) %_3.i1.i) #ATTR
          to label %cleanup1.body.i.i unwind label %terminate.i.i7.i, !noalias !ID

cleanup1.i.i:                                     ; preds = %bb2.i.i.i.i.i.i.i.i.i.i.i29.i
  %314 = landingpad { ptr, i32 }
          cleanup
  br label %cleanup1.body.i.i

cleanup1.body.i.i:                                ; preds = %cleanup1.i.i, %bb12.i.i.i, %<alloc::alloc::Global as core::alloc::Allocator>::deallocate (.exit.i5.i.i.i.i.i.i.i.i19.i), %cleanup.i.i.i.i.i.i.i.i17.i
  %eh.lpad-body.i.i = phi { ptr, i32 } [ %314, %cleanup1.i.i ], [ %.pn.ph.i.i.i, %bb12.i.i.i ], [ %309, %cleanup.i.i.i.i.i.i.i.i17.i ], [ %309, %<alloc::alloc::Global as core::alloc::Allocator>::deallocate (.exit.i5.i.i.i.i.i.i.i.i19.i) ]
; invoke <pyo3::internal::state::AttachGuard as core::ops::drop::Drop>::drop
  invoke void @<pyo3::internal::state::AttachGuard as core::ops::drop::Drop>::drop(ptr noalias nofree noundef nonnull align 4 dereferenceable(4) %guard.i.i)
          to label %common.resume.i unwind label %terminate.i3.i, !noalias !ID

terminate.i3.i:                                   ; preds = %bb6.i2.i, %cleanup1.body.i.i
  %315 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #ATTR, !noalias !ID
  unreachable

bb6.i2.i:                                         ; preds = %bb4.i
  %316 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<pyo3::err::PyErr>
  invoke void @core::ptr::drop_glue::<pyo3::err::PyErr>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(48) %283)
          to label %common.resume.i unwind label %terminate.i3.i

<pyo3::marker::Python>::attach::<purrdf_native::py_store::presentation::settled<pyo3::instance::Py<purrdf_native::py_store::query::PyQueryOutcome>, <purrdf_native::py_store::quad_store::PyQuadStore>::query_governed::{closure#0}>::{closure#0}::{closure#0}, pyo3::err::PyErr> (.exit.i): ; preds = %bb1.i.i.i.i.i.i.i.i.i.i.i30.i, %bb2.i.i.i.i.i.i.i.i.i.i.i29.i, %<alloc::alloc::Global as core::alloc::Allocator>::deallocate (.exit.i.i.i.i.i.i.i.i.i23.i), %bb3.i.i.i.i.i.i.i.i21.i, %bb5.i.i.i.i, %bb4.i6.i.i.i, %bb3.i.i31.i
  %_4.sroa.9.0.i = phi i64 [ %_4.sroa.9.0.copyload43.i, %bb3.i.i31.i ], [ %_4.sroa.9.0.copyload42.i, %bb5.i.i.i.i ], [ %_4.sroa.9.0.copyload42.i, %bb1.i.i.i.i.i.i.i.i.i.i.i30.i ], [ %_4.sroa.9.0.copyload42.i, %bb2.i.i.i.i.i.i.i.i.i.i.i29.i ], [ %_4.sroa.9.0.copyload42.i, %bb3.i.i.i.i.i.i.i.i21.i ], [ %_4.sroa.9.0.copyload42.i, %<alloc::alloc::Global as core::alloc::Allocator>::deallocate (.exit.i.i.i.i.i.i.i.i.i23.i) ], [ %_13.sroa.7.0.copyload.i.i.i, %bb4.i6.i.i.i ]
  %_4.sroa.6.0.i = phi i64 [ %_4.sroa.6.0.copyload34.i, %bb3.i.i31.i ], [ %_4.sroa.6.0.copyload33.i, %bb5.i.i.i.i ], [ %_4.sroa.6.0.copyload33.i, %bb1.i.i.i.i.i.i.i.i.i.i.i30.i ], [ %_4.sroa.6.0.copyload33.i, %bb2.i.i.i.i.i.i.i.i.i.i.i29.i ], [ %_4.sroa.6.0.copyload33.i, %bb3.i.i.i.i.i.i.i.i21.i ], [ %_4.sroa.6.0.copyload33.i, %<alloc::alloc::Global as core::alloc::Allocator>::deallocate (.exit.i.i.i.i.i.i.i.i.i23.i) ], [ %_13.sroa.4.0.copyload.i8.i.i, %bb4.i6.i.i.i ]
  %317 = phi <2 x ptr> [ %288, %bb3.i.i31.i ], [ %303, %bb5.i.i.i.i ], [ %303, %bb1.i.i.i.i.i.i.i.i.i.i.i30.i ], [ %303, %bb2.i.i.i.i.i.i.i.i.i.i.i29.i ], [ %303, %bb3.i.i.i.i.i.i.i.i21.i ], [ %303, %<alloc::alloc::Global as core::alloc::Allocator>::deallocate (.exit.i.i.i.i.i.i.i.i.i23.i) ], [ %302, %bb4.i6.i.i.i ]
  call void @llvm.lifetime.end.p0(ptr nonnull %_3.i1.i), !noalias !ID
; call <pyo3::internal::state::AttachGuard as core::ops::drop::Drop>::drop
  call void @<pyo3::internal::state::AttachGuard as core::ops::drop::Drop>::drop(ptr noalias nofree noundef nonnull align 4 dereferenceable(4) %guard.i.i), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %guard.i.i), !noalias !ID
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %283, ptr noundef nonnull align 8 dereferenceable(16) %_4.sroa.0.i, i64 16, i1 false), !noalias !ID
  %_4.sroa.6.0..sroa_idx.i = getelementptr inbounds nuw i8, ptr %_0, i64 24
  store i64 %_4.sroa.6.0.i, ptr %_4.sroa.6.0..sroa_idx.i, align 8, !alias.scope !ID, !noalias !ID
  %_4.sroa.7.0..sroa_idx.i = getelementptr inbounds nuw i8, ptr %_0, i64 32
  store <2 x ptr> %317, ptr %_4.sroa.7.0..sroa_idx.i, align 8, !alias.scope !ID, !noalias !ID
  %_4.sroa.9.0..sroa_idx.i = getelementptr inbounds nuw i8, ptr %_0, i64 48
  store i64 %_4.sroa.9.0.i, ptr %_4.sroa.9.0..sroa_idx.i, align 8, !alias.scope !ID, !noalias !ID
  store i64 1, ptr %_0, align 8, !alias.scope !ID, !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_4.sroa.0.i)
  br label %purrdf_native::py_store::presentation::settled::<pyo3::instance::Py<purrdf_native::py_store::query::PyQueryOutcome>, <purrdf_native::py_store::quad_store::PyQuadStore>::query_governed::{closure#0}> (.exit)

purrdf_native::py_store::presentation::settled::<pyo3::instance::Py<purrdf_native::py_store::query::PyQueryOutcome>, <purrdf_native::py_store::quad_store::PyQuadStore>::query_governed::{closure#0}> (.exit): ; preds = %<purrdf_native::py_store::quad_store::PyQuadStore>::query_governed::{closure#0} (.exit.i), %<pyo3::marker::Python>::attach::<purrdf_native::py_store::presentation::settled<pyo3::instance::Py<purrdf_native::py_store::query::PyQueryOutcome>, <purrdf_native::py_store::quad_store::PyQuadStore>::query_governed::{closure#0}>::{closure#0}::{closure#0}, pyo3::err::PyErr> (.exit.i)
  call void @llvm.lifetime.end.p0(ptr nonnull %_22)
  ret void
}

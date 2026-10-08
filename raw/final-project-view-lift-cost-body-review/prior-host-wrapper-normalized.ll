define void @<purrdf_native::py_store::quad_store::PyQuadStore>::__pymethod_query__(ptr dead_on_unwind noalias nofree noundef writable writeonly sret([56 x i8]) align 8 captures(none) dereferenceable(56) %result, ptr noundef %_slf, ptr nofree noundef readonly captures(address) %_args, i64 noundef %_nargs, ptr noundef %_kwargs) unnamed_addr #ATTR personality ptr @rust_eh_personality !guid !ID {
start:
  %_4.i.i.i.i207 = alloca [56 x i8], align 8
  %_9.i.i208 = alloca [48 x i8], align 8
  %_4.sroa.10.i.i209 = alloca [40 x i8], align 8
  %_4.i.i.i.i190 = alloca [56 x i8], align 8
  %_9.i.i191 = alloca [48 x i8], align 8
  %_4.i.i.i.i169 = alloca [56 x i8], align 8
  %_9.i.i170 = alloca [48 x i8], align 8
  %_4.sroa.10.i.i171 = alloca [40 x i8], align 8
  %_9.i.i148 = alloca [48 x i8], align 8
  %_9.i.i127 = alloca [48 x i8], align 8
  %_9.i.i106 = alloca [48 x i8], align 8
  %_4.i.i.i.i90 = alloca [56 x i8], align 8
  %_9.i.i91 = alloca [48 x i8], align 8
  %_4.i.i.i.i66 = alloca [56 x i8], align 8
  %_9.i.i67 = alloca [48 x i8], align 8
  %_4.sroa.10.i.i68 = alloca [40 x i8], align 8
  %_4.i.i.i.i = alloca [56 x i8], align 8
  %_9.i.i56 = alloca [48 x i8], align 8
  %_4.sroa.10.i.i = alloca [40 x i8], align 8
  %_9.i.i = alloca [48 x i8], align 8
  %_9.i = alloca [48 x i8], align 8
  %_4.i = alloca [56 x i8], align 8
  %_26 = alloca [56 x i8], align 8
  %0 = getelementptr inbounds nuw i8, ptr %_26, i64 16
  %val16 = alloca [24 x i8], align 8
  %_106 = alloca [56 x i8], align 8
  %_105.sroa.5 = alloca [48 x i8], align 8
  %_99 = alloca [56 x i8], align 8
  %_92 = alloca [56 x i8], align 8
  %_91.sroa.5 = alloca [48 x i8], align 8
  %_90 = alloca [24 x i8], align 8
  %_84 = alloca [56 x i8], align 8
  %_77 = alloca [56 x i8], align 8
  %_70 = alloca [56 x i8], align 8
  %_63 = alloca [56 x i8], align 8
  %_62.sroa.5 = alloca [48 x i8], align 8
  %_61 = alloca [48 x i8], align 8
  %_55 = alloca [56 x i8], align 8
  %_54.sroa.5 = alloca [48 x i8], align 8
  %_53 = alloca [24 x i8], align 8
  %_47 = alloca [56 x i8], align 8
  %_46.sroa.5 = alloca [48 x i8], align 8
  %_45 = alloca [24 x i8], align 8
  %_39 = alloca [56 x i8], align 8
  %_32 = alloca [56 x i8], align 8
  %ret = alloca [56 x i8], align 8
  %holder_7 = alloca [8 x i8], align 8
  %holder_6 = alloca [8 x i8], align 8
  %holder_5 = alloca [8 x i8], align 8
  %holder_1 = alloca [8 x i8], align 8
  %_8 = alloca [56 x i8], align 8
  %output = alloca [88 x i8], align 8
  call void @llvm.lifetime.start.p0(ptr nonnull %output)
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(88) %output, i8 0, i64 88, i1 false)
  call void @llvm.lifetime.start.p0(ptr nonnull %_8)
; call <pyo3::impl_::extract_argument::FunctionDescription>::extract_arguments_fastcall::<pyo3::impl_::extract_argument::NoVarargs, pyo3::impl_::extract_argument::NoVarkeywords>
  call fastcc void @<pyo3::impl_::extract_argument::FunctionDescription>::extract_arguments_fastcall::<pyo3::impl_::extract_argument::NoVarargs, pyo3::impl_::extract_argument::NoVarkeywords>(ptr noalias nofree noundef align 8 captures(none) dereferenceable(56) %_8, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(80) @alloc_f1016e7258a9778c8e4642ffd2ad0599, ptr noundef %_args, i64 noundef %_nargs, ptr noundef %_kwargs, ptr noalias nofree noundef nonnull align 8 %output, i64 noundef 11)
  %_116 = load i64, ptr %_8, align 8, !range !ID, !noundef !ID
  %1 = trunc nuw i64 %_116 to i1
  br i1 %1, label %bb37, label %bb38

bb37:                                             ; preds = %start
  %2 = getelementptr inbounds nuw i8, ptr %_8, i64 8
  %3 = getelementptr inbounds nuw i8, ptr %result, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %3, ptr noundef nonnull align 8 dereferenceable(48) %2, i64 48, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %_8)
  store i64 1, ptr %result, align 8
  br label %bb26

bb38:                                             ; preds = %start
  call void @llvm.lifetime.end.p0(ptr nonnull %_8)
  call void @llvm.lifetime.start.p0(ptr nonnull %holder_1)
  store ptr null, ptr %holder_1, align 8
  call void @llvm.lifetime.start.p0(ptr nonnull %holder_5)
  store ptr null, ptr %holder_5, align 8
  call void @llvm.lifetime.start.p0(ptr nonnull %holder_6)
  store ptr null, ptr %holder_6, align 8
  call void @llvm.lifetime.start.p0(ptr nonnull %holder_7)
  store ptr null, ptr %holder_7, align 8
  call void @llvm.lifetime.start.p0(ptr nonnull %ret)
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %_slf) ]
  %_0.i.i.i.i = getelementptr inbounds nuw i8, ptr %_slf, i64 312
; call <pyo3::pycell::impl_::BorrowChecker as pyo3::pycell::impl_::PyClassBorrowChecker>::try_borrow
  %_3.i.i46 = call noundef zeroext i1 @<pyo3::pycell::impl_::BorrowChecker as pyo3::pycell::impl_::PyClassBorrowChecker>::try_borrow(ptr noundef nonnull align 8 %_0.i.i.i.i)
  br i1 %_3.i.i46, label %bb5.i, label %bb40

bb5.i:                                            ; preds = %bb38
  %4 = getelementptr inbounds nuw i8, ptr %_26, i64 8
; call <pyo3::err::PyErr as core::convert::From<pyo3::pycell::PyBorrowError>>::from
  call void @<pyo3::err::PyErr as core::convert::From<pyo3::pycell::PyBorrowError>>::from(ptr noalias nofree noundef nonnull sret([48 x i8]) align 8 captures(none) dereferenceable(48) %4)
  %_125.sroa.0.0.copyload = load ptr, ptr %4, align 8
  %5 = getelementptr inbounds nuw i8, ptr %result, i64 8
  store ptr %_125.sroa.0.0.copyload, ptr %5, align 8
  %_128.sroa.4.0..sroa_idx = getelementptr inbounds nuw i8, ptr %result, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %_128.sroa.4.0..sroa_idx, ptr noundef nonnull align 8 dereferenceable(40) %0, i64 40, i1 false)
  store i64 1, ptr %result, align 8
  call void @llvm.lifetime.end.p0(ptr nonnull %ret)
  br label %core::ptr::drop_glue::<core::option::Option<pyo3::pyclass::guard::PyClassGuard<purrdf_native::py_store::quad_store::PyQuadStore>>> (.exit246)

bb27.thread293:                                   ; preds = %bb3.i.i62, %bb2.i.i.i.i, %.noexc54, %bb3.i.i, %bb3.i, %bb40
  %lpad.thr_comm291 = landingpad { ptr, i32 }
          cleanup
  br label %bb2.i

bb2.i:                                            ; preds = %bb35, %cleanup25, %bb27.thread293
  %.pn42288 = phi { ptr, i32 } [ %lpad.thr_comm291, %bb27.thread293 ], [ %lpad.thr_comm.split-lp, %cleanup25 ], [ %.pn40.ph, %bb35 ]
  %_2.i.i.i.i = atomicrmw sub ptr %_0.i.i.i.i, i64 1 release, align 8
  resume { ptr, i32 } %.pn42288

bb40:                                             ; preds = %bb38
  %_2.i.i = getelementptr inbounds nuw i8, ptr %_slf, i64 16
  %6 = getelementptr inbounds nuw i8, ptr %_26, i64 8
  store ptr %_2.i.i, ptr %6, align 8
  store i64 0, ptr %_26, align 8
  call void @llvm.lifetime.start.p0(ptr nonnull %_32)
  %7 = load ptr, ptr %output, align 8, !nonnull !ID, !noundef !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %_4.i), !noalias !ID
; invoke <&str as pyo3::conversion::FromPyObject>::extract
  invoke void @<&str as pyo3::conversion::FromPyObject>::extract(ptr noalias nofree noundef nonnull sret([56 x i8]) align 8 captures(address) dereferenceable(56) %_4.i, ptr noundef nonnull %7)
          to label %.noexc47 unwind label %bb27.thread293

.noexc47:                                         ; preds = %bb40
  %_5.i = load i64, ptr %_4.i, align 8, !range !ID, !noalias !ID, !noundef !ID
  %8 = trunc nuw i64 %_5.i to i1
  %9 = getelementptr inbounds nuw i8, ptr %_4.i, i64 8
  br i1 %8, label %bb3.i, label %bb42, !prof !ID

bb3.i:                                            ; preds = %.noexc47
  call void @llvm.lifetime.start.p0(ptr nonnull %_9.i), !noalias !ID
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %_9.i, ptr noundef nonnull align 8 dereferenceable(48) %9, i64 48, i1 false), !noalias !ID
  %10 = getelementptr inbounds nuw i8, ptr %_32, i64 8
; invoke pyo3::impl_::extract_argument::argument_extraction_error
  invoke void @pyo3::impl_::extract_argument::argument_extraction_error(ptr noalias nofree noundef nonnull sret([48 x i8]) align 8 captures(none) dereferenceable(48) %10, ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @alloc_da5c5f922604d9376dbdf48c863f8565, i64 noundef 5, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(48) %_9.i)
          to label %bb41 unwind label %bb27.thread293

bb41:                                             ; preds = %bb3.i
  call void @llvm.lifetime.end.p0(ptr nonnull %_9.i), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_4.i), !noalias !ID
  %_134.sroa.0.0.copyload = load ptr, ptr %10, align 8
  %_134.sroa.4.0..sroa_idx = getelementptr inbounds nuw i8, ptr %_32, i64 16
  %_134.sroa.4.0.copyload = load i64, ptr %_134.sroa.4.0..sroa_idx, align 8
  %_134.sroa.5.0..sroa_idx = getelementptr inbounds nuw i8, ptr %_32, i64 24
  %_137.sroa.5.0..sroa_idx = getelementptr inbounds nuw i8, ptr %result, i64 24
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %_137.sroa.5.0..sroa_idx, ptr noundef nonnull align 8 dereferenceable(32) %_134.sroa.5.0..sroa_idx, i64 32, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %_32)
  %11 = getelementptr inbounds nuw i8, ptr %result, i64 8
  store ptr %_134.sroa.0.0.copyload, ptr %11, align 8
  %_137.sroa.4.0..sroa_idx = getelementptr inbounds nuw i8, ptr %result, i64 16
  store i64 %_134.sroa.4.0.copyload, ptr %_137.sroa.4.0..sroa_idx, align 8
  store i64 1, ptr %result, align 8
  br label %bb2.i243

bb42:                                             ; preds = %.noexc47
  %value.0.i = load ptr, ptr %9, align 8, !noalias !ID, !nonnull !ID, !noundef !ID
  %12 = getelementptr inbounds nuw i8, ptr %_4.i, i64 16
  %value.1.i = load i64, ptr %12, align 8, !noalias !ID, !noundef !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_4.i), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_32)
  call void @llvm.lifetime.start.p0(ptr nonnull %_39)
  %13 = getelementptr inbounds nuw i8, ptr %output, i64 8
  %_40 = load ptr, ptr %13, align 8, !noundef !ID
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %.not.i = icmp eq ptr %_40, null
  br i1 %.not.i, label %bb44, label %bb3.i49

bb3.i49:                                          ; preds = %bb42
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %_15.i.i.i = call noundef ptr @Py_GetConstantBorrowed(i32 noundef 0) #ATTR, !noalias !ID
  %_3.i.i.i = icmp eq ptr %_15.i.i.i, %_40
  br i1 %_3.i.i.i, label %bb44, label %bb2.i.i.i50

bb2.i.i.i50:                                      ; preds = %bb3.i49
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %14 = getelementptr inbounds nuw i8, ptr %_40, i64 8
  %_5.i.i.i.i.i.i.i = load ptr, ptr %14, align 8, !noalias !ID, !noundef !ID
  %_6.i.i.i.i.i.i.i = call noundef i64 @PyType_GetFlags(ptr noundef %_5.i.i.i.i.i.i.i) #ATTR, !noalias !ID
  %_8.i.i.i.i.i.i.i = and i64 %_6.i.i.i.i.i.i.i, 536870912
  %_7.i.i.not.i.i.i.i.i = icmp eq i64 %_8.i.i.i.i.i.i.i, 0
  br i1 %_7.i.i.not.i.i.i.i.i, label %bb3.i.i, label %bb9.i.i.i

bb9.i.i.i:                                        ; preds = %bb2.i.i.i50
  store ptr %_40, ptr %holder_1, align 8, !alias.scope !ID, !noalias !ID
  br label %bb44

bb3.i.i:                                          ; preds = %bb2.i.i.i50
  call void @_Py_IncRef(ptr noundef nonnull @PyDict_Type) #ATTR, !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %_9.i.i), !noalias !ID
; invoke <pyo3::err::PyErr as core::convert::From<pyo3::err::cast_error::CastError>>::from
  invoke void @<pyo3::err::PyErr as core::convert::From<pyo3::err::cast_error::CastError>>::from(ptr noalias nofree noundef nonnull sret([48 x i8]) align 8 captures(none) dereferenceable(48) %_9.i.i, ptr noundef nonnull %_40, ptr noundef nonnull @PyDict_Type)
          to label %.noexc54 unwind label %bb27.thread293

.noexc54:                                         ; preds = %bb3.i.i
  %15 = getelementptr inbounds nuw i8, ptr %_39, i64 8
; invoke pyo3::impl_::extract_argument::argument_extraction_error
  invoke void @pyo3::impl_::extract_argument::argument_extraction_error(ptr noalias nofree noundef nonnull sret([48 x i8]) align 8 captures(none) dereferenceable(48) %15, ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @alloc_f98cafa4d6296fc0caeb65b9f5347e73, i64 noundef range(i64 7, 21) 13, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(48) %_9.i.i)
          to label %bb43 unwind label %bb27.thread293

bb43:                                             ; preds = %.noexc54
  call void @llvm.lifetime.end.p0(ptr nonnull %_9.i.i), !noalias !ID
  %_141.sroa.0.0.copyload = load ptr, ptr %15, align 8
  %_141.sroa.4.0..sroa_idx = getelementptr inbounds nuw i8, ptr %_39, i64 16
  %_144.sroa.4.0..sroa_idx = getelementptr inbounds nuw i8, ptr %result, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %_144.sroa.4.0..sroa_idx, ptr noundef nonnull align 8 dereferenceable(40) %_141.sroa.4.0..sroa_idx, i64 40, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %_39)
  %16 = getelementptr inbounds nuw i8, ptr %result, i64 8
  store ptr %_141.sroa.0.0.copyload, ptr %16, align 8
  store i64 1, ptr %result, align 8
  br label %bb2.i243

bb44:                                             ; preds = %bb42, %bb3.i49, %bb9.i.i.i
  %_140 = phi ptr [ %holder_1, %bb9.i.i.i ], [ null, %bb3.i49 ], [ null, %bb42 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %_39)
  call void @llvm.lifetime.start.p0(ptr nonnull %_45)
  call void @llvm.lifetime.start.p0(ptr nonnull %_46.sroa.5)
  call void @llvm.lifetime.start.p0(ptr nonnull %_47)
  %17 = getelementptr inbounds nuw i8, ptr %output, i64 16
  %_48 = load ptr, ptr %17, align 8, !noundef !ID
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %.not.i57 = icmp eq ptr %_48, null
  br i1 %.not.i57, label %bb6.thread, label %bb3.i58

bb3.i58:                                          ; preds = %bb44
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  call void @llvm.lifetime.start.p0(ptr nonnull %_4.sroa.10.i.i)
  %_9.i.i.i.i = call noundef ptr @Py_GetConstantBorrowed(i32 noundef 0) #ATTR, !noalias !ID
  %_2.i.i.i.i59 = icmp eq ptr %_9.i.i.i.i, %_48
  br i1 %_2.i.i.i.i59, label %bb6, label %bb2.i.i.i.i

bb2.i.i.i.i:                                      ; preds = %bb3.i58
  call void @llvm.lifetime.start.p0(ptr nonnull %_4.i.i.i.i), !noalias !ID
; invoke <alloc::vec::Vec<alloc::string::String> as pyo3::conversion::FromPyObject>::extract
  invoke fastcc void @<alloc::vec::Vec<alloc::string::String> as pyo3::conversion::FromPyObject>::extract(ptr noalias nofree noundef align 8 captures(address) dereferenceable(56) %_4.i.i.i.i, ptr noundef nonnull %_48)
          to label %.noexc64 unwind label %bb27.thread293

.noexc64:                                         ; preds = %bb2.i.i.i.i
  %_13.i.i.i.i = load i64, ptr %_4.i.i.i.i, align 8, !range !ID, !noalias !ID, !noundef !ID
  %18 = trunc nuw i64 %_13.i.i.i.i to i1
  %19 = getelementptr inbounds nuw i8, ptr %_4.i.i.i.i, i64 8
  %_4.sroa.5.8.copyload1.i.i = load i64, ptr %19, align 8, !noalias !ID
  %_4.sroa.10.8..sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %_4.i.i.i.i, i64 16
  br i1 %18, label %bb3.i.i62, label %<core::option::Option<alloc::vec::Vec<alloc::string::String>> as pyo3::impl_::extract_argument::PyFunctionArgument<true>>::extract (.exit.i.i)

<core::option::Option<alloc::vec::Vec<alloc::string::String>> as pyo3::impl_::extract_argument::PyFunctionArgument<true>>::extract (.exit.i.i): ; preds = %.noexc64
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %_4.sroa.10.i.i, ptr noundef nonnull align 8 dereferenceable(16) %_4.sroa.10.8..sroa_idx.i.i, i64 16, i1 false), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_4.i.i.i.i), !noalias !ID
  br label %bb6

bb3.i.i62:                                        ; preds = %.noexc64
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %_4.sroa.10.i.i, ptr noundef nonnull align 8 dereferenceable(40) %_4.sroa.10.8..sroa_idx.i.i, i64 40, i1 false), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_4.i.i.i.i), !noalias !ID
  %e.sroa.4.0._9.sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %_9.i.i56, i64 8
  call void @llvm.lifetime.start.p0(ptr nonnull %_9.i.i56), !noalias !ID
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %e.sroa.4.0._9.sroa_idx.i.i, ptr noundef nonnull align 8 dereferenceable(40) %_4.sroa.10.i.i, i64 40, i1 false), !noalias !ID
  store i64 %_4.sroa.5.8.copyload1.i.i, ptr %_9.i.i56, align 8, !alias.scope !ID, !noalias !ID
  %20 = getelementptr inbounds nuw i8, ptr %_47, i64 8
; invoke pyo3::impl_::extract_argument::argument_extraction_error
  invoke void @pyo3::impl_::extract_argument::argument_extraction_error(ptr noalias nofree noundef nonnull sret([48 x i8]) align 8 captures(none) dereferenceable(48) %20, ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @alloc_908475b3e6cbafcd4d8e4fa6ac07f48c, i64 noundef range(i64 9, 23) 20, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(48) %_9.i.i56)
          to label %bb45 unwind label %bb27.thread293

bb6.thread:                                       ; preds = %bb44
  %21 = getelementptr inbounds nuw i8, ptr %_47, i64 8
  store i64 -1, ptr %21, align 8, !alias.scope !ID, !noalias !ID
  br label %bb46

bb6:                                              ; preds = %bb3.i58, %<core::option::Option<alloc::vec::Vec<alloc::string::String>> as pyo3::impl_::extract_argument::PyFunctionArgument<true>>::extract (.exit.i.i)
  %_4.sroa.5.17.i.i = phi i64 [ %_4.sroa.5.8.copyload1.i.i, %<core::option::Option<alloc::vec::Vec<alloc::string::String>> as pyo3::impl_::extract_argument::PyFunctionArgument<true>>::extract (.exit.i.i) ], [ -1, %bb3.i58 ]
  %value.sroa.4.0..sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %_47, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %value.sroa.4.0..sroa_idx.i.i, ptr noundef nonnull align 8 dereferenceable(16) %_4.sroa.10.i.i, i64 16, i1 false), !noalias !ID
  %22 = getelementptr inbounds nuw i8, ptr %_47, i64 8
  store i64 %_4.sroa.5.17.i.i, ptr %22, align 8, !alias.scope !ID, !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_4.sroa.10.i.i)
  br label %bb46

bb45:                                             ; preds = %bb3.i.i62
  call void @llvm.lifetime.end.p0(ptr nonnull %_9.i.i56), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_4.sroa.10.i.i)
  %23 = getelementptr inbounds nuw i8, ptr %_47, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %_46.sroa.5, ptr noundef nonnull align 8 dereferenceable(48) %23, i64 48, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %_47)
  %24 = getelementptr inbounds nuw i8, ptr %result, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %24, ptr noundef nonnull align 8 dereferenceable(48) %_46.sroa.5, i64 48, i1 false)
  store i64 1, ptr %result, align 8
  call void @llvm.lifetime.end.p0(ptr nonnull %_46.sroa.5)
  br label %bb23.thread284

bb46:                                             ; preds = %bb6, %bb6.thread
  %25 = getelementptr inbounds nuw i8, ptr %_47, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %_46.sroa.5, ptr noundef nonnull align 8 dereferenceable(24) %25, i64 24, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %_47)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %_45, ptr noundef nonnull align 8 dereferenceable(24) %_46.sroa.5, i64 24, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %_46.sroa.5)
  call void @llvm.lifetime.start.p0(ptr nonnull %_53)
  call void @llvm.lifetime.start.p0(ptr nonnull %_54.sroa.5)
  call void @llvm.lifetime.start.p0(ptr nonnull %_55)
  %26 = getelementptr inbounds nuw i8, ptr %output, i64 24
  %_56 = load ptr, ptr %26, align 8, !noundef !ID
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %.not.i69 = icmp eq ptr %_56, null
  br i1 %.not.i69, label %bb7.thread, label %bb3.i70

bb3.i70:                                          ; preds = %bb46
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  call void @llvm.lifetime.start.p0(ptr nonnull %_4.sroa.10.i.i68)
  %_9.i.i.i.i71 = call noundef ptr @Py_GetConstantBorrowed(i32 noundef 0) #ATTR, !noalias !ID
  %_2.i.i.i.i72 = icmp eq ptr %_9.i.i.i.i71, %_56
  br i1 %_2.i.i.i.i72, label %bb7, label %bb2.i.i.i.i73

bb2.i.i.i.i73:                                    ; preds = %bb3.i70
  call void @llvm.lifetime.start.p0(ptr nonnull %_4.i.i.i.i66), !noalias !ID
; invoke <alloc::vec::Vec<alloc::string::String> as pyo3::conversion::FromPyObject>::extract
  invoke fastcc void @<alloc::vec::Vec<alloc::string::String> as pyo3::conversion::FromPyObject>::extract(ptr noalias nofree noundef align 8 captures(address) dereferenceable(56) %_4.i.i.i.i66, ptr noundef nonnull %_56)
          to label %.noexc87 unwind label %cleanup19

.noexc87:                                         ; preds = %bb2.i.i.i.i73
  %_13.i.i.i.i74 = load i64, ptr %_4.i.i.i.i66, align 8, !range !ID, !noalias !ID, !noundef !ID
  %27 = trunc nuw i64 %_13.i.i.i.i74 to i1
  %28 = getelementptr inbounds nuw i8, ptr %_4.i.i.i.i66, i64 8
  %_4.sroa.5.8.copyload1.i.i75 = load i64, ptr %28, align 8, !noalias !ID
  %_4.sroa.10.8..sroa_idx.i.i76 = getelementptr inbounds nuw i8, ptr %_4.i.i.i.i66, i64 16
  br i1 %27, label %bb3.i.i84, label %<core::option::Option<alloc::vec::Vec<alloc::string::String>> as pyo3::impl_::extract_argument::PyFunctionArgument<true>>::extract (.exit.i.i77)

<core::option::Option<alloc::vec::Vec<alloc::string::String>> as pyo3::impl_::extract_argument::PyFunctionArgument<true>>::extract (.exit.i.i77): ; preds = %.noexc87
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %_4.sroa.10.i.i68, ptr noundef nonnull align 8 dereferenceable(16) %_4.sroa.10.8..sroa_idx.i.i76, i64 16, i1 false), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_4.i.i.i.i66), !noalias !ID
  br label %bb7

bb3.i.i84:                                        ; preds = %.noexc87
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %_4.sroa.10.i.i68, ptr noundef nonnull align 8 dereferenceable(40) %_4.sroa.10.8..sroa_idx.i.i76, i64 40, i1 false), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_4.i.i.i.i66), !noalias !ID
  %e.sroa.4.0._9.sroa_idx.i.i85 = getelementptr inbounds nuw i8, ptr %_9.i.i67, i64 8
  call void @llvm.lifetime.start.p0(ptr nonnull %_9.i.i67), !noalias !ID
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %e.sroa.4.0._9.sroa_idx.i.i85, ptr noundef nonnull align 8 dereferenceable(40) %_4.sroa.10.i.i68, i64 40, i1 false), !noalias !ID
  store i64 %_4.sroa.5.8.copyload1.i.i75, ptr %_9.i.i67, align 8, !alias.scope !ID, !noalias !ID
  %29 = getelementptr inbounds nuw i8, ptr %_55, i64 8
; invoke pyo3::impl_::extract_argument::argument_extraction_error
  invoke void @pyo3::impl_::extract_argument::argument_extraction_error(ptr noalias nofree noundef nonnull sret([48 x i8]) align 8 captures(none) dereferenceable(48) %29, ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @alloc_ed816c88becfff4b944384e4ed1786c6, i64 noundef range(i64 9, 23) 22, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(48) %_9.i.i67)
          to label %bb47 unwind label %cleanup19

bb7.thread:                                       ; preds = %bb46
  %30 = getelementptr inbounds nuw i8, ptr %_55, i64 8
  store i64 -1, ptr %30, align 8, !alias.scope !ID, !noalias !ID
  br label %bb48

cleanup19:                                        ; preds = %bb3.i.i84, %bb2.i.i.i.i73
  %31 = landingpad { ptr, i32 }
          cleanup
  br label %bb35

bb7:                                              ; preds = %bb3.i70, %<core::option::Option<alloc::vec::Vec<alloc::string::String>> as pyo3::impl_::extract_argument::PyFunctionArgument<true>>::extract (.exit.i.i77)
  %_4.sroa.5.17.i.i79 = phi i64 [ %_4.sroa.5.8.copyload1.i.i75, %<core::option::Option<alloc::vec::Vec<alloc::string::String>> as pyo3::impl_::extract_argument::PyFunctionArgument<true>>::extract (.exit.i.i77) ], [ -1, %bb3.i70 ]
  %value.sroa.4.0..sroa_idx.i.i80 = getelementptr inbounds nuw i8, ptr %_55, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %value.sroa.4.0..sroa_idx.i.i80, ptr noundef nonnull align 8 dereferenceable(16) %_4.sroa.10.i.i68, i64 16, i1 false), !noalias !ID
  %32 = getelementptr inbounds nuw i8, ptr %_55, i64 8
  store i64 %_4.sroa.5.17.i.i79, ptr %32, align 8, !alias.scope !ID, !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_4.sroa.10.i.i68)
  br label %bb48

bb47:                                             ; preds = %bb3.i.i84
  call void @llvm.lifetime.end.p0(ptr nonnull %_9.i.i67), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_4.sroa.10.i.i68)
  %33 = getelementptr inbounds nuw i8, ptr %_55, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %_54.sroa.5, ptr noundef nonnull align 8 dereferenceable(48) %33, i64 48, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %_55)
  %34 = getelementptr inbounds nuw i8, ptr %result, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %34, ptr noundef nonnull align 8 dereferenceable(48) %_54.sroa.5, i64 48, i1 false)
  store i64 1, ptr %result, align 8
  call void @llvm.lifetime.end.p0(ptr nonnull %_54.sroa.5)
  br label %bb21

bb48:                                             ; preds = %bb7, %bb7.thread
  %35 = getelementptr inbounds nuw i8, ptr %_55, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %_54.sroa.5, ptr noundef nonnull align 8 dereferenceable(24) %35, i64 24, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %_55)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %_53, ptr noundef nonnull align 8 dereferenceable(24) %_54.sroa.5, i64 24, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %_54.sroa.5)
  call void @llvm.lifetime.start.p0(ptr nonnull %_61)
  call void @llvm.lifetime.start.p0(ptr nonnull %_62.sroa.5)
  call void @llvm.lifetime.start.p0(ptr nonnull %_63)
  %36 = getelementptr inbounds nuw i8, ptr %output, i64 32
  %_64 = load ptr, ptr %36, align 8, !noundef !ID
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %.not.i92 = icmp eq ptr %_64, null
  br i1 %.not.i92, label %bb8.thread, label %bb3.i93

bb3.i93:                                          ; preds = %bb48
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  call void @llvm.lifetime.start.p0(ptr nonnull %_4.i.i.i.i90), !noalias !ID
  %37 = getelementptr inbounds nuw i8, ptr %_4.i.i.i.i90, i64 16
  %_9.i.i.i.i94 = call noundef ptr @Py_GetConstantBorrowed(i32 noundef 0) #ATTR, !noalias !ID
  %_2.i.i.i.i95 = icmp eq ptr %_9.i.i.i.i94, %_64
  br i1 %_2.i.i.i.i95, label %bb8, label %<core::option::Option<(alloc::string::String, alloc::string::String)> as pyo3::impl_::extract_argument::PyFunctionArgument<true>>::extract (.exit.i.i)

<core::option::Option<(alloc::string::String, alloc::string::String)> as pyo3::impl_::extract_argument::PyFunctionArgument<true>>::extract (.exit.i.i): ; preds = %bb3.i93
; invoke <(alloc::string::String, alloc::string::String) as pyo3::conversion::FromPyObject>::extract
  invoke fastcc void @<(alloc::string::String, alloc::string::String) as pyo3::conversion::FromPyObject>::extract(ptr noalias nofree noundef align 8 captures(none) dereferenceable(56) %_4.i.i.i.i90, ptr noundef nonnull %_64)
          to label %.noexc104 unwind label %cleanup20

.noexc104:                                        ; preds = %<core::option::Option<(alloc::string::String, alloc::string::String)> as pyo3::impl_::extract_argument::PyFunctionArgument<true>>::extract (.exit.i.i)
  %_13.i.i.i.i96 = load i64, ptr %_4.i.i.i.i90, align 8, !range !ID, !noalias !ID, !noundef !ID
  %38 = getelementptr inbounds nuw i8, ptr %_4.i.i.i.i90, i64 8
  %_4.sroa.5.8.copyload2.i.i = load i64, ptr %38, align 8, !noalias !ID
  %39 = trunc nuw i64 %_13.i.i.i.i96 to i1
  br i1 %39, label %bb3.i.i101, label %bb8, !prof !ID

bb3.i.i101:                                       ; preds = %.noexc104
  %e.sroa.4.0._9.sroa_idx.i.i102 = getelementptr inbounds nuw i8, ptr %_9.i.i91, i64 8
  call void @llvm.lifetime.start.p0(ptr nonnull %_9.i.i91), !noalias !ID
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %e.sroa.4.0._9.sroa_idx.i.i102, ptr noundef nonnull align 8 dereferenceable(40) %37, i64 40, i1 false), !noalias !ID
  store i64 %_4.sroa.5.8.copyload2.i.i, ptr %_9.i.i91, align 8, !alias.scope !ID, !noalias !ID
  %40 = getelementptr inbounds nuw i8, ptr %_63, i64 8
; invoke pyo3::impl_::extract_argument::argument_extraction_error
  invoke void @pyo3::impl_::extract_argument::argument_extraction_error(ptr noalias nofree noundef nonnull sret([48 x i8]) align 8 captures(none) dereferenceable(48) %40, ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @alloc_93df691527a5f68f324bb7548b51c806, i64 noundef range(i64 20, 22) 21, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(48) %_9.i.i91)
          to label %bb49 unwind label %cleanup20

bb8.thread:                                       ; preds = %bb48
  %41 = getelementptr inbounds nuw i8, ptr %_63, i64 8
  store i64 -1, ptr %41, align 8, !alias.scope !ID, !noalias !ID
  br label %bb50

cleanup20:                                        ; preds = %bb3.i.i101, %<core::option::Option<(alloc::string::String, alloc::string::String)> as pyo3::impl_::extract_argument::PyFunctionArgument<true>>::extract (.exit.i.i)
  %42 = landingpad { ptr, i32 }
          cleanup
  br label %bb33

bb8:                                              ; preds = %bb3.i93, %.noexc104
  %_4.sroa.5.05.i.i = phi i64 [ %_4.sroa.5.8.copyload2.i.i, %.noexc104 ], [ -1, %bb3.i93 ]
  %value.sroa.4.0..sroa_idx.i.i98 = getelementptr inbounds nuw i8, ptr %_63, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %value.sroa.4.0..sroa_idx.i.i98, ptr noundef nonnull align 8 dereferenceable(40) %37, i64 40, i1 false), !noalias !ID
  %43 = getelementptr inbounds nuw i8, ptr %_63, i64 8
  store i64 %_4.sroa.5.05.i.i, ptr %43, align 8, !alias.scope !ID, !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_4.i.i.i.i90), !noalias !ID
  br label %bb50

bb49:                                             ; preds = %bb3.i.i101
  call void @llvm.lifetime.end.p0(ptr nonnull %_9.i.i91), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_4.i.i.i.i90), !noalias !ID
  %44 = getelementptr inbounds nuw i8, ptr %_63, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %_62.sroa.5, ptr noundef nonnull align 8 dereferenceable(48) %44, i64 48, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %_63)
  %45 = getelementptr inbounds nuw i8, ptr %result, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %45, ptr noundef nonnull align 8 dereferenceable(48) %_62.sroa.5, i64 48, i1 false)
  store i64 1, ptr %result, align 8
  call void @llvm.lifetime.end.p0(ptr nonnull %_62.sroa.5)
  br label %bb20

bb50:                                             ; preds = %bb8, %bb8.thread
  %46 = getelementptr inbounds nuw i8, ptr %_63, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %_62.sroa.5, ptr noundef nonnull align 8 dereferenceable(48) %46, i64 48, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %_63)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %_61, ptr noundef nonnull align 8 dereferenceable(48) %_62.sroa.5, i64 48, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %_62.sroa.5)
  call void @llvm.lifetime.start.p0(ptr nonnull %_70)
  %47 = getelementptr inbounds nuw i8, ptr %output, i64 40
  %_71 = load ptr, ptr %47, align 8, !noundef !ID
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %.not.i107 = icmp eq ptr %_71, null
  br i1 %.not.i107, label %bb52, label %bb3.i108

bb3.i108:                                         ; preds = %bb50
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %_15.i.i.i109 = call noundef ptr @Py_GetConstantBorrowed(i32 noundef 0) #ATTR, !noalias !ID
  %_3.i.i.i110 = icmp eq ptr %_15.i.i.i109, %_71
  br i1 %_3.i.i.i110, label %bb52, label %bb2.i.i.i111

bb2.i.i.i111:                                     ; preds = %bb3.i108
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %48 = getelementptr inbounds nuw i8, ptr %_71, i64 8
  %_5.i.i.i.i.i.i.i112 = load ptr, ptr %48, align 8, !noalias !ID, !noundef !ID
  %_6.i.i.i.i.i.i.i113 = call noundef i64 @PyType_GetFlags(ptr noundef %_5.i.i.i.i.i.i.i112) #ATTR, !noalias !ID
  %_8.i.i.i.i.i.i.i114 = and i64 %_6.i.i.i.i.i.i.i113, 536870912
  %_7.i.i.not.i.i.i.i.i115 = icmp eq i64 %_8.i.i.i.i.i.i.i114, 0
  br i1 %_7.i.i.not.i.i.i.i.i115, label %bb3.i.i121, label %bb9.i.i.i116

bb9.i.i.i116:                                     ; preds = %bb2.i.i.i111
  store ptr %_71, ptr %holder_5, align 8, !alias.scope !ID, !noalias !ID
  br label %bb52

bb3.i.i121:                                       ; preds = %bb2.i.i.i111
  call void @_Py_IncRef(ptr noundef nonnull @PyDict_Type) #ATTR, !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %_9.i.i106), !noalias !ID
; invoke <pyo3::err::PyErr as core::convert::From<pyo3::err::cast_error::CastError>>::from
  invoke void @<pyo3::err::PyErr as core::convert::From<pyo3::err::cast_error::CastError>>::from(ptr noalias nofree noundef nonnull sret([48 x i8]) align 8 captures(none) dereferenceable(48) %_9.i.i106, ptr noundef nonnull %_71, ptr noundef nonnull @PyDict_Type)
          to label %.noexc124 unwind label %cleanup21

.noexc124:                                        ; preds = %bb3.i.i121
  %49 = getelementptr inbounds nuw i8, ptr %_70, i64 8
; invoke pyo3::impl_::extract_argument::argument_extraction_error
  invoke void @pyo3::impl_::extract_argument::argument_extraction_error(ptr noalias nofree noundef nonnull sret([48 x i8]) align 8 captures(none) dereferenceable(48) %49, ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @alloc_9865433ad06614f46384a300efcb7195, i64 noundef range(i64 7, 21) 9, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(48) %_9.i.i106)
          to label %bb51 unwind label %cleanup21

cleanup21:                                        ; preds = %bb3.i.i185, %bb2.i.i.i.i176, %.noexc166, %bb3.i.i163, %.noexc145, %bb3.i.i142, %.noexc124, %bb3.i.i121
  %50 = landingpad { ptr, i32 }
          cleanup
  br label %bb31

bb51:                                             ; preds = %.noexc124
  call void @llvm.lifetime.end.p0(ptr nonnull %_9.i.i106), !noalias !ID
  %_169.sroa.0.0.copyload = load ptr, ptr %49, align 8
  %_169.sroa.4.0..sroa_idx = getelementptr inbounds nuw i8, ptr %_70, i64 16
  %_172.sroa.4.0..sroa_idx = getelementptr inbounds nuw i8, ptr %result, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %_172.sroa.4.0..sroa_idx, ptr noundef nonnull align 8 dereferenceable(40) %_169.sroa.4.0..sroa_idx, i64 40, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %_70)
  %51 = getelementptr inbounds nuw i8, ptr %result, i64 8
  store ptr %_169.sroa.0.0.copyload, ptr %51, align 8
  store i64 1, ptr %result, align 8
  br label %bb19

bb52:                                             ; preds = %bb50, %bb3.i108, %bb9.i.i.i116
  %_168 = phi ptr [ %holder_5, %bb9.i.i.i116 ], [ null, %bb3.i108 ], [ null, %bb50 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %_70)
  call void @llvm.lifetime.start.p0(ptr nonnull %_77)
  %52 = getelementptr inbounds nuw i8, ptr %output, i64 48
  %_78 = load ptr, ptr %52, align 8, !noundef !ID
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %.not.i128 = icmp eq ptr %_78, null
  br i1 %.not.i128, label %bb54, label %bb3.i129

bb3.i129:                                         ; preds = %bb52
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %_15.i.i.i130 = call noundef ptr @Py_GetConstantBorrowed(i32 noundef 0) #ATTR, !noalias !ID
  %_3.i.i.i131 = icmp eq ptr %_15.i.i.i130, %_78
  br i1 %_3.i.i.i131, label %bb54, label %bb2.i.i.i132

bb2.i.i.i132:                                     ; preds = %bb3.i129
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %53 = getelementptr inbounds nuw i8, ptr %_78, i64 8
  %_5.i.i.i.i.i.i.i133 = load ptr, ptr %53, align 8, !noalias !ID, !noundef !ID
  %_6.i.i.i.i.i.i.i134 = call noundef i64 @PyType_GetFlags(ptr noundef %_5.i.i.i.i.i.i.i133) #ATTR, !noalias !ID
  %_8.i.i.i.i.i.i.i135 = and i64 %_6.i.i.i.i.i.i.i134, 536870912
  %_7.i.i.not.i.i.i.i.i136 = icmp eq i64 %_8.i.i.i.i.i.i.i135, 0
  br i1 %_7.i.i.not.i.i.i.i.i136, label %bb3.i.i142, label %bb9.i.i.i137

bb9.i.i.i137:                                     ; preds = %bb2.i.i.i132
  store ptr %_78, ptr %holder_6, align 8, !alias.scope !ID, !noalias !ID
  br label %bb54

bb3.i.i142:                                       ; preds = %bb2.i.i.i132
  call void @_Py_IncRef(ptr noundef nonnull @PyDict_Type) #ATTR, !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %_9.i.i127), !noalias !ID
; invoke <pyo3::err::PyErr as core::convert::From<pyo3::err::cast_error::CastError>>::from
  invoke void @<pyo3::err::PyErr as core::convert::From<pyo3::err::cast_error::CastError>>::from(ptr noalias nofree noundef nonnull sret([48 x i8]) align 8 captures(none) dereferenceable(48) %_9.i.i127, ptr noundef nonnull %_78, ptr noundef nonnull @PyDict_Type)
          to label %.noexc145 unwind label %cleanup21

.noexc145:                                        ; preds = %bb3.i.i142
  %54 = getelementptr inbounds nuw i8, ptr %_77, i64 8
; invoke pyo3::impl_::extract_argument::argument_extraction_error
  invoke void @pyo3::impl_::extract_argument::argument_extraction_error(ptr noalias nofree noundef nonnull sret([48 x i8]) align 8 captures(none) dereferenceable(48) %54, ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @alloc_fcd42a11584ddca09ee4bceee5bc0781, i64 noundef range(i64 7, 21) 20, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(48) %_9.i.i127)
          to label %bb53 unwind label %cleanup21

bb53:                                             ; preds = %.noexc145
  call void @llvm.lifetime.end.p0(ptr nonnull %_9.i.i127), !noalias !ID
  %_176.sroa.0.0.copyload = load ptr, ptr %54, align 8
  %_176.sroa.4.0..sroa_idx = getelementptr inbounds nuw i8, ptr %_77, i64 16
  %_179.sroa.4.0..sroa_idx = getelementptr inbounds nuw i8, ptr %result, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %_179.sroa.4.0..sroa_idx, ptr noundef nonnull align 8 dereferenceable(40) %_176.sroa.4.0..sroa_idx, i64 40, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %_77)
  %55 = getelementptr inbounds nuw i8, ptr %result, i64 8
  store ptr %_176.sroa.0.0.copyload, ptr %55, align 8
  store i64 1, ptr %result, align 8
  br label %bb19

bb54:                                             ; preds = %bb52, %bb3.i129, %bb9.i.i.i137
  %_175 = phi ptr [ %holder_6, %bb9.i.i.i137 ], [ null, %bb3.i129 ], [ null, %bb52 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %_77)
  call void @llvm.lifetime.start.p0(ptr nonnull %_84)
  %56 = getelementptr inbounds nuw i8, ptr %output, i64 56
  %_85 = load ptr, ptr %56, align 8, !noundef !ID
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %.not.i149 = icmp eq ptr %_85, null
  br i1 %.not.i149, label %bb56, label %bb3.i150

bb3.i150:                                         ; preds = %bb54
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %_15.i.i.i151 = call noundef ptr @Py_GetConstantBorrowed(i32 noundef 0) #ATTR, !noalias !ID
  %_3.i.i.i152 = icmp eq ptr %_15.i.i.i151, %_85
  br i1 %_3.i.i.i152, label %bb56, label %bb2.i.i.i153

bb2.i.i.i153:                                     ; preds = %bb3.i150
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %57 = getelementptr inbounds nuw i8, ptr %_85, i64 8
  %_5.i.i.i.i.i.i.i154 = load ptr, ptr %57, align 8, !noalias !ID, !noundef !ID
  %_6.i.i.i.i.i.i.i155 = call noundef i64 @PyType_GetFlags(ptr noundef %_5.i.i.i.i.i.i.i154) #ATTR, !noalias !ID
  %_8.i.i.i.i.i.i.i156 = and i64 %_6.i.i.i.i.i.i.i155, 536870912
  %_7.i.i.not.i.i.i.i.i157 = icmp eq i64 %_8.i.i.i.i.i.i.i156, 0
  br i1 %_7.i.i.not.i.i.i.i.i157, label %bb3.i.i163, label %bb9.i.i.i158

bb9.i.i.i158:                                     ; preds = %bb2.i.i.i153
  store ptr %_85, ptr %holder_7, align 8, !alias.scope !ID, !noalias !ID
  br label %bb56

bb3.i.i163:                                       ; preds = %bb2.i.i.i153
  call void @_Py_IncRef(ptr noundef nonnull @PyDict_Type) #ATTR, !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %_9.i.i148), !noalias !ID
; invoke <pyo3::err::PyErr as core::convert::From<pyo3::err::cast_error::CastError>>::from
  invoke void @<pyo3::err::PyErr as core::convert::From<pyo3::err::cast_error::CastError>>::from(ptr noalias nofree noundef nonnull sret([48 x i8]) align 8 captures(none) dereferenceable(48) %_9.i.i148, ptr noundef nonnull %_85, ptr noundef nonnull @PyDict_Type)
          to label %.noexc166 unwind label %cleanup21

.noexc166:                                        ; preds = %bb3.i.i163
  %58 = getelementptr inbounds nuw i8, ptr %_84, i64 8
; invoke pyo3::impl_::extract_argument::argument_extraction_error
  invoke void @pyo3::impl_::extract_argument::argument_extraction_error(ptr noalias nofree noundef nonnull sret([48 x i8]) align 8 captures(none) dereferenceable(48) %58, ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @alloc_2f742b7bf960932f7bae3af5028d09e0, i64 noundef range(i64 7, 21) 14, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(48) %_9.i.i148)
          to label %bb55 unwind label %cleanup21

bb55:                                             ; preds = %.noexc166
  call void @llvm.lifetime.end.p0(ptr nonnull %_9.i.i148), !noalias !ID
  %_183.sroa.0.0.copyload = load ptr, ptr %58, align 8
  %_183.sroa.4.0..sroa_idx = getelementptr inbounds nuw i8, ptr %_84, i64 16
  %_186.sroa.4.0..sroa_idx = getelementptr inbounds nuw i8, ptr %result, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %_186.sroa.4.0..sroa_idx, ptr noundef nonnull align 8 dereferenceable(40) %_183.sroa.4.0..sroa_idx, i64 40, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %_84)
  %59 = getelementptr inbounds nuw i8, ptr %result, i64 8
  store ptr %_183.sroa.0.0.copyload, ptr %59, align 8
  store i64 1, ptr %result, align 8
  br label %bb19

bb56:                                             ; preds = %bb54, %bb3.i150, %bb9.i.i.i158
  %_182 = phi ptr [ %holder_7, %bb9.i.i.i158 ], [ null, %bb3.i150 ], [ null, %bb54 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %_84)
  call void @llvm.lifetime.start.p0(ptr nonnull %_90)
  call void @llvm.lifetime.start.p0(ptr nonnull %_91.sroa.5)
  call void @llvm.lifetime.start.p0(ptr nonnull %_92)
  %60 = getelementptr inbounds nuw i8, ptr %output, i64 64
  %_93 = load ptr, ptr %60, align 8, !noundef !ID
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %.not.i172 = icmp eq ptr %_93, null
  br i1 %.not.i172, label %bb12.thread, label %bb3.i173

bb3.i173:                                         ; preds = %bb56
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  call void @llvm.lifetime.start.p0(ptr nonnull %_4.sroa.10.i.i171)
  %_9.i.i.i.i174 = call noundef ptr @Py_GetConstantBorrowed(i32 noundef 0) #ATTR, !noalias !ID
  %_2.i.i.i.i175 = icmp eq ptr %_9.i.i.i.i174, %_93
  br i1 %_2.i.i.i.i175, label %bb12, label %bb2.i.i.i.i176

bb2.i.i.i.i176:                                   ; preds = %bb3.i173
  call void @llvm.lifetime.start.p0(ptr nonnull %_4.i.i.i.i169), !noalias !ID
; invoke <alloc::string::String as pyo3::conversion::FromPyObject>::extract
  invoke void @<alloc::string::String as pyo3::conversion::FromPyObject>::extract(ptr noalias nofree noundef nonnull sret([56 x i8]) align 8 captures(none) dereferenceable(56) %_4.i.i.i.i169, ptr noundef nonnull %_93)
          to label %.noexc188 unwind label %cleanup21

.noexc188:                                        ; preds = %bb2.i.i.i.i176
  %_13.i.i.i.i177 = load i64, ptr %_4.i.i.i.i169, align 8, !range !ID, !noalias !ID, !noundef !ID
  %61 = trunc nuw i64 %_13.i.i.i.i177 to i1
  %62 = getelementptr inbounds nuw i8, ptr %_4.i.i.i.i169, i64 8
  %_4.sroa.5.8.copyload1.i.i178 = load i64, ptr %62, align 8, !noalias !ID
  %_4.sroa.10.8..sroa_idx.i.i179 = getelementptr inbounds nuw i8, ptr %_4.i.i.i.i169, i64 16
  br i1 %61, label %bb3.i.i185, label %<core::option::Option<alloc::string::String> as pyo3::impl_::extract_argument::PyFunctionArgument<true>>::extract (.exit.i.i)

<core::option::Option<alloc::string::String> as pyo3::impl_::extract_argument::PyFunctionArgument<true>>::extract (.exit.i.i): ; preds = %.noexc188
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %_4.sroa.10.i.i171, ptr noundef nonnull align 8 dereferenceable(16) %_4.sroa.10.8..sroa_idx.i.i179, i64 16, i1 false), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_4.i.i.i.i169), !noalias !ID
  br label %bb12

bb3.i.i185:                                       ; preds = %.noexc188
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %_4.sroa.10.i.i171, ptr noundef nonnull align 8 dereferenceable(40) %_4.sroa.10.8..sroa_idx.i.i179, i64 40, i1 false), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_4.i.i.i.i169), !noalias !ID
  %e.sroa.4.0._9.sroa_idx.i.i186 = getelementptr inbounds nuw i8, ptr %_9.i.i170, i64 8
  call void @llvm.lifetime.start.p0(ptr nonnull %_9.i.i170), !noalias !ID
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %e.sroa.4.0._9.sroa_idx.i.i186, ptr noundef nonnull align 8 dereferenceable(40) %_4.sroa.10.i.i171, i64 40, i1 false), !noalias !ID
  store i64 %_4.sroa.5.8.copyload1.i.i178, ptr %_9.i.i170, align 8, !alias.scope !ID, !noalias !ID
  %63 = getelementptr inbounds nuw i8, ptr %_92, i64 8
; invoke pyo3::impl_::extract_argument::argument_extraction_error
  invoke void @pyo3::impl_::extract_argument::argument_extraction_error(ptr noalias nofree noundef nonnull sret([48 x i8]) align 8 captures(none) dereferenceable(48) %63, ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @alloc_589d20955c603729090b6aaccd02222f, i64 noundef range(i64 4, 20) 19, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(48) %_9.i.i170)
          to label %bb57 unwind label %cleanup21

bb12.thread:                                      ; preds = %bb56
  %64 = getelementptr inbounds nuw i8, ptr %_92, i64 8
  store i64 -1, ptr %64, align 8, !alias.scope !ID, !noalias !ID
  br label %bb58

bb12:                                             ; preds = %bb3.i173, %<core::option::Option<alloc::string::String> as pyo3::impl_::extract_argument::PyFunctionArgument<true>>::extract (.exit.i.i)
  %_4.sroa.5.17.i.i181 = phi i64 [ %_4.sroa.5.8.copyload1.i.i178, %<core::option::Option<alloc::string::String> as pyo3::impl_::extract_argument::PyFunctionArgument<true>>::extract (.exit.i.i) ], [ -1, %bb3.i173 ]
  %value.sroa.4.0..sroa_idx.i.i182 = getelementptr inbounds nuw i8, ptr %_92, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %value.sroa.4.0..sroa_idx.i.i182, ptr noundef nonnull align 8 dereferenceable(16) %_4.sroa.10.i.i171, i64 16, i1 false), !noalias !ID
  %65 = getelementptr inbounds nuw i8, ptr %_92, i64 8
  store i64 %_4.sroa.5.17.i.i181, ptr %65, align 8, !alias.scope !ID, !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_4.sroa.10.i.i171)
  br label %bb58

bb57:                                             ; preds = %bb3.i.i185
  call void @llvm.lifetime.end.p0(ptr nonnull %_9.i.i170), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_4.sroa.10.i.i171)
  %66 = getelementptr inbounds nuw i8, ptr %_92, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %_91.sroa.5, ptr noundef nonnull align 8 dereferenceable(48) %66, i64 48, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %_92)
  %67 = getelementptr inbounds nuw i8, ptr %result, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %67, ptr noundef nonnull align 8 dereferenceable(48) %_91.sroa.5, i64 48, i1 false)
  store i64 1, ptr %result, align 8
  call void @llvm.lifetime.end.p0(ptr nonnull %_91.sroa.5)
  br label %bb18

bb58:                                             ; preds = %bb12, %bb12.thread
  %68 = getelementptr inbounds nuw i8, ptr %_92, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %_91.sroa.5, ptr noundef nonnull align 8 dereferenceable(24) %68, i64 24, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %_92)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %_90, ptr noundef nonnull align 8 dereferenceable(24) %_91.sroa.5, i64 24, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %_91.sroa.5)
  call void @llvm.lifetime.start.p0(ptr nonnull %_99)
  %69 = getelementptr inbounds nuw i8, ptr %output, i64 72
  %_100 = load ptr, ptr %69, align 8, !noundef !ID
  %.not.i192 = icmp eq ptr %_100, null
  br i1 %.not.i192, label %bb60, label %bb3.i193

bb3.i193:                                         ; preds = %bb58
  %_9.i.i.i.i194 = call noundef ptr @Py_GetConstantBorrowed(i32 noundef 0) #ATTR, !noalias !ID
  %_2.i.i.i.i195 = icmp eq ptr %_9.i.i.i.i194, %_100
  br i1 %_2.i.i.i.i195, label %bb60, label %bb2.i.i.i.i196

bb2.i.i.i.i196:                                   ; preds = %bb3.i193
  call void @llvm.lifetime.start.p0(ptr nonnull %_4.i.i.i.i190), !noalias !ID
; invoke <&str as pyo3::conversion::FromPyObject>::extract
  invoke void @<&str as pyo3::conversion::FromPyObject>::extract(ptr noalias nofree noundef nonnull sret([56 x i8]) align 8 captures(address) dereferenceable(56) %_4.i.i.i.i190, ptr noundef nonnull %_100)
          to label %.noexc205 unwind label %bb29

.noexc205:                                        ; preds = %bb2.i.i.i.i196
  %_13.i.i.i.i197 = load i64, ptr %_4.i.i.i.i190, align 8, !range !ID, !noalias !ID, !noundef !ID
  %70 = trunc nuw i64 %_13.i.i.i.i197 to i1
  %71 = getelementptr inbounds nuw i8, ptr %_4.i.i.i.i190, i64 8
  %_4.sroa.5.8.copyload1.i.i198 = load ptr, ptr %71, align 8, !noalias !ID
  %_4.sroa.9.8..sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %_4.i.i.i.i190, i64 16
  %_4.sroa.9.8.copyload2.i.i = load i64, ptr %_4.sroa.9.8..sroa_idx.i.i, align 8, !noalias !ID
  br i1 %70, label %bb3.i.i202, label %<core::option::Option<&str> as pyo3::impl_::extract_argument::PyFunctionArgument<true>>::extract (.exit.i.i)

<core::option::Option<&str> as pyo3::impl_::extract_argument::PyFunctionArgument<true>>::extract (.exit.i.i): ; preds = %.noexc205
  call void @llvm.lifetime.end.p0(ptr nonnull %_4.i.i.i.i190), !noalias !ID
  br label %bb60

bb3.i.i202:                                       ; preds = %.noexc205
  %_4.sroa.11.8..sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %_4.i.i.i.i190, i64 24
  %e.sroa.5.0._9.sroa_idx.i.i = getelementptr inbounds nuw i8, ptr %_9.i.i191, i64 16
  call void @llvm.lifetime.start.p0(ptr nonnull %_9.i.i191), !noalias !ID
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %e.sroa.5.0._9.sroa_idx.i.i, ptr noundef nonnull align 8 dereferenceable(32) %_4.sroa.11.8..sroa_idx.i.i, i64 32, i1 false), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_4.i.i.i.i190), !noalias !ID
  store ptr %_4.sroa.5.8.copyload1.i.i198, ptr %_9.i.i191, align 8, !alias.scope !ID, !noalias !ID
  %e.sroa.4.0._9.sroa_idx.i.i203 = getelementptr inbounds nuw i8, ptr %_9.i.i191, i64 8
  store i64 %_4.sroa.9.8.copyload2.i.i, ptr %e.sroa.4.0._9.sroa_idx.i.i203, align 8, !alias.scope !ID, !noalias !ID
  %72 = getelementptr inbounds nuw i8, ptr %_99, i64 8
; invoke pyo3::impl_::extract_argument::argument_extraction_error
  invoke void @pyo3::impl_::extract_argument::argument_extraction_error(ptr noalias nofree noundef nonnull sret([48 x i8]) align 8 captures(none) dereferenceable(48) %72, ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @alloc_6c43a98927907a5f8539235d09232c7e, i64 noundef range(i64 3, 16) 11, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(48) %_9.i.i191)
          to label %bb59 unwind label %bb29

cleanup25:                                        ; preds = %bb62
  %lpad.thr_comm.split-lp = landingpad { ptr, i32 }
          cleanup
  br label %bb2.i

bb59:                                             ; preds = %bb3.i.i202
  call void @llvm.lifetime.end.p0(ptr nonnull %_9.i.i191), !noalias !ID
  %_197.sroa.0.0.copyload = load ptr, ptr %72, align 8
  %_197.sroa.4.0..sroa_idx = getelementptr inbounds nuw i8, ptr %_99, i64 16
  %_197.sroa.4.0.copyload = load i64, ptr %_197.sroa.4.0..sroa_idx, align 8
  %_197.sroa.5.0..sroa_idx = getelementptr inbounds nuw i8, ptr %_99, i64 24
  %_200.sroa.5.0..sroa_idx = getelementptr inbounds nuw i8, ptr %result, i64 24
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %_200.sroa.5.0..sroa_idx, ptr noundef nonnull align 8 dereferenceable(32) %_197.sroa.5.0..sroa_idx, i64 32, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %_99)
  %73 = getelementptr inbounds nuw i8, ptr %result, i64 8
  store ptr %_197.sroa.0.0.copyload, ptr %73, align 8
  %_200.sroa.4.0..sroa_idx = getelementptr inbounds nuw i8, ptr %result, i64 16
  store i64 %_197.sroa.4.0.copyload, ptr %_200.sroa.4.0..sroa_idx, align 8
  store i64 1, ptr %result, align 8
  br label %bb17

bb60:                                             ; preds = %bb58, %bb3.i193, %<core::option::Option<&str> as pyo3::impl_::extract_argument::PyFunctionArgument<true>>::extract (.exit.i.i)
  %_196.1 = phi i64 [ undef, %bb3.i193 ], [ %_4.sroa.9.8.copyload2.i.i, %<core::option::Option<&str> as pyo3::impl_::extract_argument::PyFunctionArgument<true>>::extract (.exit.i.i) ], [ undef, %bb58 ]
  %_196.0 = phi ptr [ null, %bb3.i193 ], [ %_4.sroa.5.8.copyload1.i.i198, %<core::option::Option<&str> as pyo3::impl_::extract_argument::PyFunctionArgument<true>>::extract (.exit.i.i) ], [ null, %bb58 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %_99)
  call void @llvm.lifetime.start.p0(ptr nonnull %_105.sroa.5)
  call void @llvm.lifetime.start.p0(ptr nonnull %_106)
  %74 = getelementptr inbounds nuw i8, ptr %output, i64 80
  %_107 = load ptr, ptr %74, align 8, !noundef !ID
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %.not.i210 = icmp eq ptr %_107, null
  br i1 %.not.i210, label %bb14.thread, label %bb3.i211

bb3.i211:                                         ; preds = %bb60
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  call void @llvm.lifetime.start.p0(ptr nonnull %_4.sroa.10.i.i209)
  %_9.i.i.i.i212 = call noundef ptr @Py_GetConstantBorrowed(i32 noundef 0) #ATTR, !noalias !ID
  %_2.i.i.i.i213 = icmp eq ptr %_9.i.i.i.i212, %_107
  br i1 %_2.i.i.i.i213, label %bb14, label %bb2.i.i.i.i214

bb2.i.i.i.i214:                                   ; preds = %bb3.i211
  call void @llvm.lifetime.start.p0(ptr nonnull %_4.i.i.i.i207), !noalias !ID
; invoke <alloc::string::String as pyo3::conversion::FromPyObject>::extract
  invoke void @<alloc::string::String as pyo3::conversion::FromPyObject>::extract(ptr noalias nofree noundef nonnull sret([56 x i8]) align 8 captures(none) dereferenceable(56) %_4.i.i.i.i207, ptr noundef nonnull %_107)
          to label %.noexc228 unwind label %bb29

.noexc228:                                        ; preds = %bb2.i.i.i.i214
  %_13.i.i.i.i215 = load i64, ptr %_4.i.i.i.i207, align 8, !range !ID, !noalias !ID, !noundef !ID
  %75 = trunc nuw i64 %_13.i.i.i.i215 to i1
  %76 = getelementptr inbounds nuw i8, ptr %_4.i.i.i.i207, i64 8
  %_4.sroa.5.8.copyload1.i.i216 = load i64, ptr %76, align 8, !noalias !ID
  %_4.sroa.10.8..sroa_idx.i.i217 = getelementptr inbounds nuw i8, ptr %_4.i.i.i.i207, i64 16
  br i1 %75, label %bb3.i.i225, label %<core::option::Option<alloc::string::String> as pyo3::impl_::extract_argument::PyFunctionArgument<true>>::extract (.exit.i.i218)

<core::option::Option<alloc::string::String> as pyo3::impl_::extract_argument::PyFunctionArgument<true>>::extract (.exit.i.i218): ; preds = %.noexc228
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %_4.sroa.10.i.i209, ptr noundef nonnull align 8 dereferenceable(16) %_4.sroa.10.8..sroa_idx.i.i217, i64 16, i1 false), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_4.i.i.i.i207), !noalias !ID
  br label %bb14

bb3.i.i225:                                       ; preds = %.noexc228
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %_4.sroa.10.i.i209, ptr noundef nonnull align 8 dereferenceable(40) %_4.sroa.10.8..sroa_idx.i.i217, i64 40, i1 false), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_4.i.i.i.i207), !noalias !ID
  %e.sroa.4.0._9.sroa_idx.i.i226 = getelementptr inbounds nuw i8, ptr %_9.i.i208, i64 8
  call void @llvm.lifetime.start.p0(ptr nonnull %_9.i.i208), !noalias !ID
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %e.sroa.4.0._9.sroa_idx.i.i226, ptr noundef nonnull align 8 dereferenceable(40) %_4.sroa.10.i.i209, i64 40, i1 false), !noalias !ID
  store i64 %_4.sroa.5.8.copyload1.i.i216, ptr %_9.i.i208, align 8, !alias.scope !ID, !noalias !ID
  %77 = getelementptr inbounds nuw i8, ptr %_106, i64 8
; invoke pyo3::impl_::extract_argument::argument_extraction_error
  invoke void @pyo3::impl_::extract_argument::argument_extraction_error(ptr noalias nofree noundef nonnull sret([48 x i8]) align 8 captures(none) dereferenceable(48) %77, ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @alloc_8497b6da903267d7e7d13af19cddca66, i64 noundef range(i64 4, 20) 8, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(48) %_9.i.i208)
          to label %bb61 unwind label %bb29

bb14.thread:                                      ; preds = %bb60
  %78 = getelementptr inbounds nuw i8, ptr %_106, i64 8
  store i64 -1, ptr %78, align 8, !alias.scope !ID, !noalias !ID
  br label %bb62

bb14:                                             ; preds = %bb3.i211, %<core::option::Option<alloc::string::String> as pyo3::impl_::extract_argument::PyFunctionArgument<true>>::extract (.exit.i.i218)
  %_4.sroa.5.17.i.i220 = phi i64 [ %_4.sroa.5.8.copyload1.i.i216, %<core::option::Option<alloc::string::String> as pyo3::impl_::extract_argument::PyFunctionArgument<true>>::extract (.exit.i.i218) ], [ -1, %bb3.i211 ]
  %value.sroa.4.0..sroa_idx.i.i221 = getelementptr inbounds nuw i8, ptr %_106, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %value.sroa.4.0..sroa_idx.i.i221, ptr noundef nonnull align 8 dereferenceable(16) %_4.sroa.10.i.i209, i64 16, i1 false), !noalias !ID
  %79 = getelementptr inbounds nuw i8, ptr %_106, i64 8
  store i64 %_4.sroa.5.17.i.i220, ptr %79, align 8, !alias.scope !ID, !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_4.sroa.10.i.i209)
  br label %bb62

bb61:                                             ; preds = %bb3.i.i225
  call void @llvm.lifetime.end.p0(ptr nonnull %_9.i.i208), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %_4.sroa.10.i.i209)
  %80 = getelementptr inbounds nuw i8, ptr %_106, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %_105.sroa.5, ptr noundef nonnull align 8 dereferenceable(48) %80, i64 48, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %_106)
  %81 = getelementptr inbounds nuw i8, ptr %result, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %81, ptr noundef nonnull align 8 dereferenceable(48) %_105.sroa.5, i64 48, i1 false)
  store i64 1, ptr %result, align 8
  call void @llvm.lifetime.end.p0(ptr nonnull %_105.sroa.5)
  br label %bb17

bb62:                                             ; preds = %bb14, %bb14.thread
  %82 = getelementptr inbounds nuw i8, ptr %_106, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %_105.sroa.5, ptr noundef nonnull align 8 dereferenceable(24) %82, i64 24, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %_106)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %val16, ptr noundef nonnull align 8 dereferenceable(24) %_105.sroa.5, i64 24, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %_105.sroa.5)
; invoke <purrdf_native::py_store::quad_store::PyQuadStore>::query
  invoke fastcc void @<purrdf_native::py_store::quad_store::PyQuadStore>::query(ptr noalias nofree noundef align 8 captures(address) dereferenceable(56) %ret, ptr noundef nonnull align 8 %_2.i.i, ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) %value.0.i, i64 noundef %value.1.i, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable_or_null(8) %_140, ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %_45, ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %_53, ptr noalias nofree noundef align 8 captures(address) dereferenceable(48) %_61, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable_or_null(8) %_168, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable_or_null(8) %_175, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable_or_null(8) %_182, ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %_90, ptr noalias nofree noundef readonly captures(address, read_provenance) %_196.0, i64 %_196.1, ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %val16)
          to label %bb15 unwind label %cleanup25

bb15:                                             ; preds = %bb62
  call void @llvm.lifetime.end.p0(ptr nonnull %_90)
  call void @llvm.lifetime.end.p0(ptr nonnull %_61)
  call void @llvm.lifetime.end.p0(ptr nonnull %_53)
  call void @llvm.lifetime.end.p0(ptr nonnull %_45)
  %_210 = load i64, ptr %ret, align 8, !range !ID, !noundef !ID
  %83 = trunc nuw i64 %_210 to i1
  %84 = getelementptr inbounds nuw i8, ptr %ret, i64 8
  %_212.sroa.0.0.copyload = load ptr, ptr %84, align 8
  br i1 %83, label %bb67, label %core::ptr::drop_glue::<core::option::Option<pyo3::pyclass::guard::PyClassGuard<purrdf_native::py_store::quad_store::PyQuadStore>>> (.exit234)

bb67:                                             ; preds = %bb15
  %_212.sroa.4.0..sroa_idx = getelementptr inbounds nuw i8, ptr %ret, i64 16
  %_215.sroa.4.0..sroa_idx = getelementptr inbounds nuw i8, ptr %result, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %_215.sroa.4.0..sroa_idx, ptr noundef nonnull align 8 dereferenceable(40) %_212.sroa.4.0..sroa_idx, i64 40, i1 false)
  br label %core::ptr::drop_glue::<core::option::Option<pyo3::pyclass::guard::PyClassGuard<purrdf_native::py_store::quad_store::PyQuadStore>>> (.exit234)

core::ptr::drop_glue::<core::option::Option<pyo3::pyclass::guard::PyClassGuard<purrdf_native::py_store::quad_store::PyQuadStore>>> (.exit234): ; preds = %bb15, %bb67
  %.sink = phi i64 [ 1, %bb67 ], [ 0, %bb15 ]
  %85 = getelementptr inbounds nuw i8, ptr %result, i64 8
  store ptr %_212.sroa.0.0.copyload, ptr %85, align 8
  store i64 %.sink, ptr %result, align 8
  call void @llvm.lifetime.end.p0(ptr nonnull %ret)
  %_2.i.i.i.i233 = atomicrmw sub ptr %_0.i.i.i.i, i64 1 release, align 8
  call void @llvm.lifetime.end.p0(ptr nonnull %holder_7)
  call void @llvm.lifetime.end.p0(ptr nonnull %holder_6)
  call void @llvm.lifetime.end.p0(ptr nonnull %holder_5)
  call void @llvm.lifetime.end.p0(ptr nonnull %holder_1)
  br label %bb26

bb26:                                             ; preds = %core::ptr::drop_glue::<core::option::Option<pyo3::pyclass::guard::PyClassGuard<purrdf_native::py_store::quad_store::PyQuadStore>>> (.exit246), %bb37, %core::ptr::drop_glue::<core::option::Option<pyo3::pyclass::guard::PyClassGuard<purrdf_native::py_store::quad_store::PyQuadStore>>> (.exit234)
  call void @llvm.lifetime.end.p0(ptr nonnull %output)
  ret void

bb17:                                             ; preds = %bb59, %bb61
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %86 = load i64, ptr %_90, align 8, !range !ID, !alias.scope !ID, !noundef !ID
  %87 = icmp eq i64 %86, -1
  br i1 %87, label %bb18, label %bb2.i235

bb2.i235:                                         ; preds = %bb17
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %88 = icmp eq i64 %86, 0
  br i1 %88, label %bb18, label %bb2.i.i.i4.i.i.i

bb2.i.i.i4.i.i.i:                                 ; preds = %bb2.i235
  %89 = getelementptr inbounds nuw i8, ptr %_90, i64 8
  %_1.val1.i.i = load ptr, ptr %89, align 8, !alias.scope !ID, !nonnull !ID, !noundef !ID
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %_1.val1.i.i, i64 noundef %86, i64 noundef range(i64 1, -9223372036854775807) 1) #ATTR, !noalias !ID
  br label %bb18

bb18:                                             ; preds = %bb2.i.i.i4.i.i.i, %bb2.i235, %bb17, %bb57
  call void @llvm.lifetime.end.p0(ptr nonnull %_90)
  br label %bb19

bb29:                                             ; preds = %bb3.i.i225, %bb3.i.i202, %bb2.i.i.i.i196, %bb2.i.i.i.i214
  %lpad.thr_comm = landingpad { ptr, i32 }
          cleanup
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %90 = load i64, ptr %_90, align 8, !range !ID, !alias.scope !ID, !noundef !ID
  %91 = icmp eq i64 %90, -1
  br i1 %91, label %bb31, label %bb2.i236

bb2.i236:                                         ; preds = %bb29
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %92 = icmp eq i64 %90, 0
  br i1 %92, label %bb31, label %bb2.i.i.i4.i.i.i237

bb2.i.i.i4.i.i.i237:                              ; preds = %bb2.i236
  %93 = getelementptr inbounds nuw i8, ptr %_90, i64 8
  %_1.val1.i.i238 = load ptr, ptr %93, align 8, !alias.scope !ID, !nonnull !ID, !noundef !ID
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %_1.val1.i.i238, i64 noundef %90, i64 noundef range(i64 1, -9223372036854775807) 1) #ATTR, !noalias !ID
  br label %bb31

bb19:                                             ; preds = %bb51, %bb53, %bb55, %bb18
; call core::ptr::drop_glue::<core::option::Option<purrdf_sparql_results::model::ProvenanceNamespace>>
  call fastcc void @core::ptr::drop_glue::<core::option::Option<purrdf_sparql_results::model::ProvenanceNamespace>>(ptr noalias nofree noundef align 8 dereferenceable(48) %_61)
  br label %bb20

bb20:                                             ; preds = %bb19, %bb49
  call void @llvm.lifetime.end.p0(ptr nonnull %_61)
; call core::ptr::drop_glue::<core::option::Option<alloc::vec::Vec<alloc::string::String>>>
  call fastcc void @core::ptr::drop_glue::<core::option::Option<alloc::vec::Vec<alloc::string::String>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %_53)
  br label %bb21

bb31:                                             ; preds = %cleanup21, %bb29, %bb2.i236, %bb2.i.i.i4.i.i.i237
  %.pn.ph = phi { ptr, i32 } [ %50, %cleanup21 ], [ %lpad.thr_comm, %bb29 ], [ %lpad.thr_comm, %bb2.i236 ], [ %lpad.thr_comm, %bb2.i.i.i4.i.i.i237 ]
; call core::ptr::drop_glue::<core::option::Option<purrdf_sparql_results::model::ProvenanceNamespace>>
  call fastcc void @core::ptr::drop_glue::<core::option::Option<purrdf_sparql_results::model::ProvenanceNamespace>>(ptr noalias nofree noundef align 8 dereferenceable(48) %_61) #ATTR
  br label %bb33

bb21:                                             ; preds = %bb20, %bb47
  call void @llvm.lifetime.end.p0(ptr nonnull %_53)
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %94 = load i64, ptr %_45, align 8, !range !ID, !alias.scope !ID, !noundef !ID
  %95 = icmp eq i64 %94, -1
  br i1 %95, label %bb23.thread284, label %bb2.i240

bb2.i240:                                         ; preds = %bb21
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %96 = getelementptr inbounds nuw i8, ptr %_45, i64 8
  %_1.val.i.i = load ptr, ptr %96, align 8, !alias.scope !ID, !nonnull !ID, !noundef !ID
  %97 = getelementptr inbounds nuw i8, ptr %_45, i64 16
  %_1.val1.i.i241 = load i64, ptr %97, align 8, !alias.scope !ID, !noundef !ID
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %_710.i.i.i.i = icmp eq i64 %_1.val1.i.i241, 0
  br i1 %_710.i.i.i.i, label %bb4.i.i242, label %bb5.i.i.i.i

bb5.i.i.i.i:                                      ; preds = %bb2.i240, %core::ptr::drop_glue::<alloc::string::String> (.exit.i.i.i.i)
  %_3.sroa.0.011.i.i.i.i = phi i64 [ %98, %core::ptr::drop_glue::<alloc::string::String> (.exit.i.i.i.i) ], [ 0, %bb2.i240 ]
  %_6.i.i.i.i = getelementptr inbounds nuw [24 x i8], ptr %_1.val.i.i, i64 %_3.sroa.0.011.i.i.i.i
  %98 = add nuw nsw i64 %_3.sroa.0.011.i.i.i.i, 1
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %_1.val.i.i.i.i.i = load i64, ptr %_6.i.i.i.i, align 8, !alias.scope !ID, !noalias !ID
  %99 = icmp eq i64 %_1.val.i.i.i.i.i, 0
  br i1 %99, label %core::ptr::drop_glue::<alloc::string::String> (.exit.i.i.i.i), label %bb2.i.i.i4.i.i.i.i.i.i

bb2.i.i.i4.i.i.i.i.i.i:                           ; preds = %bb5.i.i.i.i
  %100 = getelementptr inbounds nuw i8, ptr %_6.i.i.i.i, i64 8
  %_1.val1.i.i.i.i.i = load ptr, ptr %100, align 8, !alias.scope !ID, !noalias !ID, !nonnull !ID, !noundef !ID
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %_1.val1.i.i.i.i.i, i64 noundef %_1.val.i.i.i.i.i, i64 noundef range(i64 1, -9223372036854775807) 1) #ATTR, !noalias !ID
  br label %core::ptr::drop_glue::<alloc::string::String> (.exit.i.i.i.i)

core::ptr::drop_glue::<alloc::string::String> (.exit.i.i.i.i): ; preds = %bb2.i.i.i4.i.i.i.i.i.i, %bb5.i.i.i.i
  %_7.i.i.i.i = icmp eq i64 %98, %_1.val1.i.i241
  br i1 %_7.i.i.i.i, label %bb4.i.i242, label %bb5.i.i.i.i

bb4.i.i242:                                       ; preds = %core::ptr::drop_glue::<alloc::string::String> (.exit.i.i.i.i), %bb2.i240
  %101 = icmp eq i64 %94, 0
  br i1 %101, label %bb23, label %bb2.i.i.i6.i.i

bb2.i.i.i6.i.i:                                   ; preds = %bb4.i.i242
  %alloc_size.i.i.i.i7.i.i = mul nuw i64 %94, 24
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %_1.val.i.i, i64 noundef %alloc_size.i.i.i.i7.i.i, i64 noundef range(i64 1, -9223372036854775807) 8) #ATTR, !noalias !ID
  br label %bb23

bb33:                                             ; preds = %bb31, %cleanup20
  %.pn38.ph = phi { ptr, i32 } [ %.pn.ph, %bb31 ], [ %42, %cleanup20 ]
; call core::ptr::drop_glue::<core::option::Option<alloc::vec::Vec<alloc::string::String>>>
  call fastcc void @core::ptr::drop_glue::<core::option::Option<alloc::vec::Vec<alloc::string::String>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %_53) #ATTR
  br label %bb35

bb35:                                             ; preds = %bb33, %cleanup19
  %.pn40.ph = phi { ptr, i32 } [ %.pn38.ph, %bb33 ], [ %31, %cleanup19 ]
; call core::ptr::drop_glue::<core::option::Option<alloc::vec::Vec<alloc::string::String>>>
  call fastcc void @core::ptr::drop_glue::<core::option::Option<alloc::vec::Vec<alloc::string::String>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %_45) #ATTR
  br label %bb2.i

bb23.thread284:                                   ; preds = %bb45, %bb21
  call void @llvm.lifetime.end.p0(ptr nonnull %_45)
  br label %bb2.i243

bb23:                                             ; preds = %bb4.i.i242, %bb2.i.i.i6.i.i
  call void @llvm.lifetime.end.p0(ptr nonnull %_45)
  br label %bb2.i243

bb2.i243:                                         ; preds = %bb43, %bb41, %bb23, %bb23.thread284
  call void @llvm.lifetime.end.p0(ptr nonnull %ret)
  %_2.i.i.i.i245 = atomicrmw sub ptr %_0.i.i.i.i, i64 1 release, align 8
  br label %core::ptr::drop_glue::<core::option::Option<pyo3::pyclass::guard::PyClassGuard<purrdf_native::py_store::quad_store::PyQuadStore>>> (.exit246)

core::ptr::drop_glue::<core::option::Option<pyo3::pyclass::guard::PyClassGuard<purrdf_native::py_store::quad_store::PyQuadStore>>> (.exit246): ; preds = %bb5.i, %bb2.i243
  call void @llvm.lifetime.end.p0(ptr nonnull %holder_7)
  call void @llvm.lifetime.end.p0(ptr nonnull %holder_6)
  call void @llvm.lifetime.end.p0(ptr nonnull %holder_5)
  call void @llvm.lifetime.end.p0(ptr nonnull %holder_1)
  br label %bb26
}

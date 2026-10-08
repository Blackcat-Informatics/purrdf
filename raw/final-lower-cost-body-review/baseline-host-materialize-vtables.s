.Lvtable.1R:
	.quad	core::ptr::drop_glue::<<pyo3::err::PyErr>::new<pyo3::exceptions::PyKeyError, alloc::string::String>::{closure#0}>
	.asciz	"\030\000\000\000\000\000\000\000\b\000\000\000\000\000\000"
	.quad	<<pyo3::err::PyErr>::new<pyo3::exceptions::PyValueError, alloc::string::String>::{closure#0} as core::ops::function::FnOnce<(pyo3::marker::Python,)>>::call_once::{shim:vtable#0}
	.size	.Lvtable.1R, 32
.Lvtable.1J:
	.quad	core::ptr::drop_glue::<alloc::string::String>
	.asciz	"\030\000\000\000\000\000\000\000\b\000\000\000\000\000\000"
	.quad	<alloc::string::String as core::fmt::Write>::write_str
	.quad	<alloc::string::String as core::fmt::Write>::write_char
	.quad	<alloc::string::String as core::fmt::Write>::write_fmt
	.size	.Lvtable.1J, 48
.Lvtable.2c:
	.asciz	"\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\001\000\000\000\000\000\000"
	.quad	<core::fmt::Error as core::fmt::Debug>::fmt
	.size	.Lvtable.2c, 32

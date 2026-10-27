// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc.
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Admission at the shared emitter's literal construction sites.

use super::{Ctx, Memory, Object, StorageError, ToolPolicy, Value};
use crate::json_model::ToJson;
use purrdf_lex::json::record::IntoJsonNative;

pub(in crate::json_schema) trait Source: ToJson {
    fn admit(&self, policy: &mut ToolPolicy) -> bool;
    fn copy(&self, memory: &mut Memory<'_, ToolPolicy>) -> Result<Value, StorageError>;
}

impl<T: Source + ?Sized> Source for &T {
    fn admit(&self, policy: &mut ToolPolicy) -> bool {
        T::admit(self, policy)
    }
    fn copy(&self, memory: &mut Memory<'_, ToolPolicy>) -> Result<Value, StorageError> {
        T::copy(self, memory)
    }
}

macro_rules! primitive {
    ($($ty:ty),* $(,)?) => {$ (
        impl Source for $ty {
            fn admit(&self, policy: &mut ToolPolicy) -> bool { policy.charge(1) && policy.native_nodes(1) }
            fn copy(&self, memory: &mut Memory<'_, ToolPolicy>) -> Result<Value, StorageError> { (*self).into_json_native(memory) }
        }
    )*};
}
primitive!(
    bool, u8, u16, u32, u64, usize, i8, i16, i32, i64, isize, i128
);

impl Source for str {
    fn admit(&self, policy: &mut ToolPolicy) -> bool {
        policy.charge(self.len() as u64) && policy.native_nodes(1)
    }
    fn copy(&self, memory: &mut Memory<'_, ToolPolicy>) -> Result<Value, StorageError> {
        self.into_json_native(memory)
    }
}
impl Source for String {
    fn admit(&self, policy: &mut ToolPolicy) -> bool {
        self.as_str().admit(policy)
    }
    fn copy(&self, memory: &mut Memory<'_, ToolPolicy>) -> Result<Value, StorageError> {
        Source::copy(self.as_str(), memory)
    }
}
impl Source for Value {
    fn admit(&self, policy: &mut ToolPolicy) -> bool {
        super::admit_native_clone(self, policy)
    }
    fn copy(&self, memory: &mut Memory<'_, ToolPolicy>) -> Result<Value, StorageError> {
        self.clone_with_memory(memory)
    }
}
impl<T: Source> Source for [T] {
    fn admit(&self, policy: &mut ToolPolicy) -> bool {
        policy.charge(1) && policy.native_nodes(1) && self.iter().all(|value| value.admit(policy))
    }
    fn copy(&self, memory: &mut Memory<'_, ToolPolicy>) -> Result<Value, StorageError> {
        let mut values = Vec::new();
        for value in self {
            let value = value.copy(memory)?;
            memory.push(&mut values, value)?;
        }
        Ok(Value::Array(values))
    }
}
impl<T: Source> Source for Vec<T> {
    fn admit(&self, policy: &mut ToolPolicy) -> bool {
        self.as_slice().admit(policy)
    }
    fn copy(&self, memory: &mut Memory<'_, ToolPolicy>) -> Result<Value, StorageError> {
        Source::copy(self.as_slice(), memory)
    }
}
impl<T: Source> Source for Option<T> {
    fn admit(&self, policy: &mut ToolPolicy) -> bool {
        match self {
            Some(value) => value.admit(policy),
            None => policy.charge(1) && policy.native_nodes(1),
        }
    }
    fn copy(&self, memory: &mut Memory<'_, ToolPolicy>) -> Result<Value, StorageError> {
        self.as_ref()
            .map_or(Ok(Value::Null), |value| value.copy(memory))
    }
}

pub(in crate::json_schema) fn value<T: Source + ?Sized>(ctx: &mut Ctx<'_>, source: &T) -> Value {
    let Some(policy) = &mut ctx.tool else {
        return source.to_json();
    };
    if !source.admit(policy) {
        return Value::Null;
    }
    policy
        .native(|memory| source.copy(memory))
        .unwrap_or(Value::Null)
}

pub(in crate::json_schema) fn member(
    ctx: &mut Ctx<'_>,
    members: &mut Vec<(String, Value)>,
    name: &str,
    value: Value,
) {
    if let Some((_, previous)) = members.iter_mut().find(|(key, _)| key == name) {
        *previous = value;
        return;
    }
    if let Some(policy) = &mut ctx.tool {
        policy.native(|memory| {
            let name = memory.string(name)?;
            memory.push(members, (name, value))
        });
    } else {
        members.push((name.to_owned(), value));
    }
}

pub(in crate::json_schema) fn object(
    ctx: &mut Ctx<'_>,
    mut members: Vec<(String, Value)>,
) -> Value {
    if let Some(policy) = &mut ctx.tool
        && !(policy.charge(1) && policy.native_nodes(1))
    {
        return Value::Null;
    }
    members.sort_unstable_by(|a, b| a.0.cmp(&b.0));
    Value::Object(Object::from(members))
}

pub(in crate::json_schema) fn array(
    ctx: &mut Ctx<'_>,
    values: impl IntoIterator<Item = Value>,
) -> Value {
    if ctx.tool.is_some() {
        super::native_array(ctx, values)
    } else {
        Value::Array(values.into_iter().collect())
    }
}

macro_rules! emitted_json {
    // ── Array elements, accumulated as `expr,` ──────────────────────────
    (@array $ctx:expr; [$($elems:expr,)*]) => {
        [$($elems,)*]
    };
    (@array $ctx:expr; [$($elems:expr),*]) => {
        [$($elems),*]
    };
    (@array $ctx:expr; [$($elems:expr,)*] null $($rest:tt)*) => {
        $crate::json_schema::tool_arguments::emitted_json!(@array $ctx; [$($elems,)* $crate::json_schema::tool_arguments::emitted_json!($ctx; null)] $($rest)*)
    };
    (@array $ctx:expr; [$($elems:expr,)*] true $($rest:tt)*) => {
        $crate::json_schema::tool_arguments::emitted_json!(@array $ctx; [$($elems,)* $crate::json_schema::tool_arguments::emitted_json!($ctx; true)] $($rest)*)
    };
    (@array $ctx:expr; [$($elems:expr,)*] false $($rest:tt)*) => {
        $crate::json_schema::tool_arguments::emitted_json!(@array $ctx; [$($elems,)* $crate::json_schema::tool_arguments::emitted_json!($ctx; false)] $($rest)*)
    };
    (@array $ctx:expr; [$($elems:expr,)*] [$($array:tt)*] $($rest:tt)*) => {
        $crate::json_schema::tool_arguments::emitted_json!(@array $ctx; [$($elems,)* $crate::json_schema::tool_arguments::emitted_json!($ctx; [$($array)*])] $($rest)*)
    };
    (@array $ctx:expr; [$($elems:expr,)*] {$($map:tt)*} $($rest:tt)*) => {
        $crate::json_schema::tool_arguments::emitted_json!(@array $ctx; [$($elems,)* $crate::json_schema::tool_arguments::emitted_json!($ctx; {$($map)*})] $($rest)*)
    };
    (@array $ctx:expr; [$($elems:expr,)*] $next:expr, $($rest:tt)*) => {
        $crate::json_schema::tool_arguments::emitted_json!(@array $ctx; [$($elems,)* $crate::json_schema::tool_arguments::emitted_json!($ctx; $next),] $($rest)*)
    };
    (@array $ctx:expr; [$($elems:expr,)*] $last:expr) => {
        $crate::json_schema::tool_arguments::emitted_json!(@array $ctx; [$($elems,)* $crate::json_schema::tool_arguments::emitted_json!($ctx; $last)])
    };
    (@array $ctx:expr; [$($elems:expr),*] , $($rest:tt)*) => {
        $crate::json_schema::tool_arguments::emitted_json!(@array $ctx; [$($elems,)*] $($rest)*)
    };

    // ── Object members, inserted into the map `$object` ─────────────────
    (@object $ctx:expr; $object:ident () () ()) => {};
    (@object $ctx:expr; $object:ident [$($key:tt)+] ($value:expr) , $($rest:tt)*) => {
        let value = $value;
        $crate::json_schema::tool_arguments::emission::member($ctx, &mut $object, $($key)+, value);
        $crate::json_schema::tool_arguments::emitted_json!(@object $ctx; $object () ($($rest)*) ($($rest)*));
    };
    (@object $ctx:expr; $object:ident [$($key:tt)+] ($value:expr)) => {
        let value = $value;
        $crate::json_schema::tool_arguments::emission::member($ctx, &mut $object, $($key)+, value);
    };
    (@object $ctx:expr; $object:ident ($($key:tt)+) (: null $($rest:tt)*) $copy:tt) => {
        $crate::json_schema::tool_arguments::emitted_json!(@object $ctx; $object [$($key)+] ($crate::json_schema::tool_arguments::emitted_json!($ctx; null)) $($rest)*);
    };
    (@object $ctx:expr; $object:ident ($($key:tt)+) (: true $($rest:tt)*) $copy:tt) => {
        $crate::json_schema::tool_arguments::emitted_json!(@object $ctx; $object [$($key)+] ($crate::json_schema::tool_arguments::emitted_json!($ctx; true)) $($rest)*);
    };
    (@object $ctx:expr; $object:ident ($($key:tt)+) (: false $($rest:tt)*) $copy:tt) => {
        $crate::json_schema::tool_arguments::emitted_json!(@object $ctx; $object [$($key)+] ($crate::json_schema::tool_arguments::emitted_json!($ctx; false)) $($rest)*);
    };
    (@object $ctx:expr; $object:ident ($($key:tt)+) (: [$($array:tt)*] $($rest:tt)*) $copy:tt) => {
        $crate::json_schema::tool_arguments::emitted_json!(@object $ctx; $object [$($key)+] ($crate::json_schema::tool_arguments::emitted_json!($ctx; [$($array)*])) $($rest)*);
    };
    (@object $ctx:expr; $object:ident ($($key:tt)+) (: {$($map:tt)*} $($rest:tt)*) $copy:tt) => {
        $crate::json_schema::tool_arguments::emitted_json!(@object $ctx; $object [$($key)+] ($crate::json_schema::tool_arguments::emitted_json!($ctx; {$($map)*})) $($rest)*);
    };
    (@object $ctx:expr; $object:ident ($($key:tt)+) (: $value:expr , $($rest:tt)*) $copy:tt) => {
        $crate::json_schema::tool_arguments::emitted_json!(@object $ctx; $object [$($key)+] ($crate::json_schema::tool_arguments::emitted_json!($ctx; $value)) , $($rest)*);
    };
    (@object $ctx:expr; $object:ident ($($key:tt)+) (: $value:expr) $copy:tt) => {
        $crate::json_schema::tool_arguments::emitted_json!(@object $ctx; $object [$($key)+] ($crate::json_schema::tool_arguments::emitted_json!($ctx; $value)));
    };
    (@object $ctx:expr; $object:ident () (($key:expr) : $($rest:tt)*) $copy:tt) => {
        $crate::json_schema::tool_arguments::emitted_json!(@object $ctx; $object ($key) (: $($rest)*) (: $($rest)*));
    };
    (@object $ctx:expr; $object:ident ($($key:tt)*) ($tt:tt $($rest:tt)*) $copy:tt) => {
        $crate::json_schema::tool_arguments::emitted_json!(@object $ctx; $object ($($key)* $tt) ($($rest)*) ($($rest)*));
    };

    // ── Entry points ────────────────────────────────────────────────────
    ($ctx:expr; null) => {
        $crate::json_schema::tool_arguments::emission::value($ctx, &$crate::json_model::Value::Null)
    };
    ($ctx:expr; true) => {
        $crate::json_schema::tool_arguments::emission::value($ctx, &true)
    };
    ($ctx:expr; false) => {
        $crate::json_schema::tool_arguments::emission::value($ctx, &false)
    };
    ($ctx:expr; []) => {
        $crate::json_schema::tool_arguments::emission::array($ctx, [])
    };
    ($ctx:expr; [ $($tt:tt)+ ]) => {
        { let values = $crate::json_schema::tool_arguments::emitted_json!(@array $ctx; [] $($tt)+); $crate::json_schema::tool_arguments::emission::array($ctx, values) }
    };
    ($ctx:expr; {}) => {
        $crate::json_schema::tool_arguments::emission::object($ctx, ::std::vec::Vec::new())
    };
    ($ctx:expr; { $($tt:tt)+ }) => {
        {
            let mut object = ::std::vec::Vec::new();
            $crate::json_schema::tool_arguments::emitted_json!(@object $ctx; object () ($($tt)+) ($($tt)+));
            $crate::json_schema::tool_arguments::emission::object($ctx, object)
        }
    };
    ($ctx:expr; $other:expr) => {
        { let source = &$other; $crate::json_schema::tool_arguments::emission::value($ctx, source) }
    };
}

pub(in crate::json_schema) use emitted_json;

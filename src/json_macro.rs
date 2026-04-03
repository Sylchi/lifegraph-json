#[macro_export]
macro_rules! json_unexpected {
    ($unexpected:tt) => {
        compile_error!(concat!(
            "unexpected token in json! macro: ",
            stringify!($unexpected)
        ))
    };
    () => {
        compile_error!("unexpected end of json! macro invocation")
    };
}

#[macro_export]
macro_rules! json {
    ($($json:tt)+) => {
        $crate::json_internal!($($json)+)
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! json_internal {
    (@array [$($elems:expr,)*]) => {
        vec![$($elems,)*]
    };
    (@array [$($elems:expr),*]) => {
        vec![$($elems),*]
    };
    (@array [$($elems:expr,)*] null $($rest:tt)*) => {
        $crate::json_internal!(@array [$($elems,)* $crate::json_internal!(null)] $($rest)*)
    };
    (@array [$($elems:expr,)*] true $($rest:tt)*) => {
        $crate::json_internal!(@array [$($elems,)* $crate::json_internal!(true)] $($rest)*)
    };
    (@array [$($elems:expr,)*] false $($rest:tt)*) => {
        $crate::json_internal!(@array [$($elems,)* $crate::json_internal!(false)] $($rest)*)
    };
    (@array [$($elems:expr,)*] [$($array:tt)*] $($rest:tt)*) => {
        $crate::json_internal!(@array [$($elems,)* $crate::json_internal!([$($array)*])] $($rest)*)
    };
    (@array [$($elems:expr,)*] {$($map:tt)*} $($rest:tt)*) => {
        $crate::json_internal!(@array [$($elems,)* $crate::json_internal!({$($map)*})] $($rest)*)
    };
    (@array [$($elems:expr,)*] $next:expr, $($rest:tt)*) => {
        $crate::json_internal!(@array [$($elems,)* $crate::json_internal!($next),] $($rest)*)
    };
    (@array [$($elems:expr,)*] $last:expr) => {
        $crate::json_internal!(@array [$($elems,)* $crate::json_internal!($last)])
    };
    (@array [$($elems:expr),*] , $($rest:tt)*) => {
        $crate::json_internal!(@array [$($elems,)*] $($rest)*)
    };
    (@array [$($elems:expr),*] $unexpected:tt $($rest:tt)*) => {
        $crate::json_unexpected!($unexpected)
    };

    (@object $object:ident () () ()) => {};
    (@object $object:ident [$($key:tt)+] ($value:expr) , $($rest:tt)*) => {
        $object.push((($($key)+).into(), $value));
        $crate::json_internal!(@object $object () ($($rest)*) ($($rest)*));
    };
    (@object $object:ident [$($key:tt)+] ($value:expr)) => {
        $object.push((($($key)+).into(), $value));
    };
    (@object $object:ident [$($key:tt)+] ($value:expr) $unexpected:tt $($rest:tt)*) => {
        $crate::json_unexpected!($unexpected)
    };
    (@object $object:ident ($($key:tt)+) (: null $($rest:tt)*) $copy:tt) => {
        $crate::json_internal!(@object $object [$($key)+] ($crate::json_internal!(null)) $($rest)*);
    };
    (@object $object:ident ($($key:tt)+) (: true $($rest:tt)*) $copy:tt) => {
        $crate::json_internal!(@object $object [$($key)+] ($crate::json_internal!(true)) $($rest)*);
    };
    (@object $object:ident ($($key:tt)+) (: false $($rest:tt)*) $copy:tt) => {
        $crate::json_internal!(@object $object [$($key)+] ($crate::json_internal!(false)) $($rest)*);
    };
    (@object $object:ident ($($key:tt)+) (: [$($array:tt)*] $($rest:tt)*) $copy:tt) => {
        $crate::json_internal!(@object $object [$($key)+] ($crate::json_internal!([$($array)*])) $($rest)*);
    };
    (@object $object:ident ($($key:tt)+) (: {$($map:tt)*} $($rest:tt)*) $copy:tt) => {
        $crate::json_internal!(@object $object [$($key)+] ($crate::json_internal!({$($map)*})) $($rest)*);
    };
    (@object $object:ident ($($key:tt)+) (: $value:expr , $($rest:tt)*) $copy:tt) => {
        $crate::json_internal!(@object $object [$($key)+] ($crate::json_internal!($value)) , $($rest)*);
    };
    (@object $object:ident ($($key:tt)+) (: $value:expr) $copy:tt) => {
        $crate::json_internal!(@object $object [$($key)+] ($crate::json_internal!($value)));
    };
    (@object $object:ident ($($key:tt)+) (:) $copy:tt) => {
        $crate::json_internal!();
    };
    (@object $object:ident ($($key:tt)+) () $copy:tt) => {
        $crate::json_internal!();
    };
    (@object $object:ident () (: $($rest:tt)*) ($colon:tt $($copy:tt)*)) => {
        $crate::json_unexpected!($colon)
    };
    (@object $object:ident ($($key:tt)*) (, $($rest:tt)*) ($comma:tt $($copy:tt)*)) => {
        $crate::json_unexpected!($comma)
    };
    (@object $object:ident ($($key:tt)*) ($tt:tt $($rest:tt)*) ($($copy:tt)*)) => {
        $crate::json_internal!(@object $object ($($key)* $tt) ($($rest)*) ($($rest)*));
    };

    (null) => { $crate::JsonValue::Null };
    (true) => { $crate::JsonValue::Bool(true) };
    (false) => { $crate::JsonValue::Bool(false) };
    ([]) => { $crate::JsonValue::Array(vec![]) };
    ([ $($tt:tt)+ ]) => { $crate::JsonValue::Array($crate::json_internal!(@array [] $($tt)+)) };
    ({}) => { $crate::JsonValue::Object($crate::Map::new()) };
    ({ $($tt:tt)+ }) => {{
        let mut object = $crate::Map::new();
        $crate::json_internal!(@object object () ($($tt)+) ($($tt)+));
        $crate::JsonValue::Object(object)
    }};
    ($other:expr) => { $crate::JsonValue::from($other) };
}

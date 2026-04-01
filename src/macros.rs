/// Construct a CBOR array from a list of expressions.
///
/// Each element is converted into a [`Value`](crate::Value).
///
/// ```
/// # use cbor_core::{array, Value};
/// let v = array![1, "hello", true];
/// assert!(v.data_type().is_array());
/// ```
#[macro_export]
macro_rules! array {
    ($($x:expr),* $(,)?) => {
        $crate::Value::array([$($crate::Value::from($x)),*])
    };
}

/// Construct a CBOR map from a list of `key => value` pairs.
///
/// Keys and values are converted into [`Value`](crate::Value).
///
/// ```
/// # use cbor_core::{map, Value};
/// let m = map! { "x" => 1, "y" => 2 };
/// assert!(m.data_type().is_map());
/// ```
#[macro_export]
macro_rules! map {
    () => {
        $crate::Value::Map(::std::collections::BTreeMap::new())
    };
    ($($k:expr => $v:expr),+ $(,)?) => {{
        let mut map = ::std::collections::BTreeMap::new();
        $(map.insert($crate::Value::from($k), $crate::Value::from($v));)+
        $crate::Value::Map(map)
    }};
}

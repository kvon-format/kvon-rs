use std::collections::HashMap;

pub type GetterResult<T> = Result<T, ()>;

#[derive(Debug, Clone, PartialEq)]
pub enum PrimitiveValue {
	Number(f32),
	String(String),
	Boolean(bool),
	Null,
}

impl PrimitiveValue {
	pub fn is_number(&self) -> bool {
		matches!(self, Self::Number(_))
	}

	pub fn is_string(&self) -> bool {
		matches!(self, Self::String(_))
	}

	pub fn is_boolean(&self) -> bool {
		matches!(self, Self::Boolean(_))
	}

	pub fn is_null(&self) -> bool {
		matches!(self, Self::Null)
	}

	pub fn get_number(&self) -> GetterResult<f32> {
		match self {
			Self::Number(n) => Ok(*n),
			_ => Err(()),
		}
	}

	pub fn get_boolean(&self) -> GetterResult<bool> {
		match self {
			Self::Boolean(b) => Ok(*b),
			_ => Err(()),
		}
	}

	pub fn get_string(&self) -> GetterResult<&String> {
		match self {
			Self::String(s) => Ok(s),
			_ => Err(()),
		}
	}
}

impl From<f32> for PrimitiveValue {
	fn from(value: f32) -> Self {
		Self::Number(value)
	}
}

impl From<String> for PrimitiveValue {
	fn from(value: String) -> Self {
		Self::String(value.to_string())
	}
}

impl From<&str> for PrimitiveValue {
	fn from(value: &str) -> Self {
		Self::String(value.to_string())
	}
}

impl From<char> for PrimitiveValue {
	fn from(value: char) -> Self {
		Self::String(value.to_string())
	}
}

impl From<bool> for PrimitiveValue {
	fn from(value: bool) -> Self {
		Self::Boolean(value)
	}
}

/// Possible values keys can map to, or arrays contain.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
	Primitive(PrimitiveValue),
	Object(HashMap<String, Value>),
	Array(Vec<Value>),
}

impl Value {
	pub fn empty_object() -> Value {
		Value::Object(HashMap::new())
	}

	pub fn null() -> Value {
		Value::Primitive(PrimitiveValue::Null)
	}

	pub fn key_value_pair(key: impl ToString, value: impl Into<Value>) -> Self {
		let mut m = HashMap::new();
		m.insert(key.to_string(), value.into());
		Self::Object(m)
	}

	pub fn is_primitive(&self) -> bool {
		matches!(self, Self::Primitive(_))
	}

	pub fn is_object(&self) -> bool {
		matches!(self, Self::Object(_))
	}

	pub fn is_array(&self) -> bool {
		matches!(self, Self::Array(_))
	}

	pub fn get_objects(&self) -> GetterResult<&HashMap<String, Value>> {
		match self {
			Self::Object(obj) => Ok(obj),
			_ => Err(()),
		}
	}

	pub fn get_vector(&self) -> GetterResult<&Vec<Value>> {
		match self {
			Self::Array(arr) => Ok(arr),
			_ => Err(()),
		}
	}

	pub fn get_primitive(&self) -> GetterResult<&PrimitiveValue> {
		match self {
			Self::Primitive(primitive) => Ok(primitive),
			_ => Err(()),
		}
	}

	pub fn object_from_iter<K, V, T>(iter: T) -> Value
	where
		K: ToString,
		V: Into<Value>,
		T: IntoIterator<Item = (K, V)>,
	{
		Value::Object(HashMap::from_iter(
			iter.into_iter()
				.map(|(key, value)| (key.to_string(), value.into())),
		))
	}

	pub fn object_from_vec(vec: Vec<(&str, Value)>) -> Value {
		Self::object_from_iter(vec.into_iter())
	}
}

impl<T: Into<PrimitiveValue>> From<T> for Value {
	fn from(value: T) -> Self {
		Self::Primitive(value.into())
	}
}

impl From<i32> for Value {
	fn from(value: i32) -> Self {
		Self::Primitive((value as f32).into())
	}
}

/// Adapted from https://docs.rs/json/0.12.4/src/json/lib.rs.html.
#[macro_export]
macro_rules! array {
    [] => ($crate::value::new_array());

    // Handles for token tree items
    [@ITEM($( $i:expr, )*) $item:tt, $( $cont:tt )+] => {
        $crate::array!(
            @ITEM($( $i, )* $crate::value!($item), )
            $( $cont )*
        )
    };
    (@ITEM($( $i:expr, )*) $item:tt,) => ({
        $crate::array!(@END $( $i, )* $crate::value!($item), )
    });
    (@ITEM($( $i:expr, )*) $item:tt) => ({
        $crate::array!(@END $( $i, )* $crate::value!($item), )
    });

    // Handles for expression items
    [@ITEM($( $i:expr, )*) $item:expr, $( $cont:tt )+] => {
        $crate::array!(
            @ITEM($( $i, )* $crate::value!($item), )
            $( $cont )*
        )
    };
    (@ITEM($( $i:expr, )*) $item:expr,) => ({
        $crate::array!(@END $( $i, )* $crate::value!($item), )
    });
    (@ITEM($( $i:expr, )*) $item:expr) => ({
        $crate::array!(@END $( $i, )* $crate::value!($item), )
    });

    // Construct the actual array
    (@END $( $i:expr, )*) => ({
        let size = 0 $( + {let _ = &$i; 1} )*;
        let mut array = Vec::with_capacity(size);

        $(
            array.push($i.into());
        )*

        $crate::value::Value::Array(array)
    });

    // Entry point to the macro
    ($( $cont:tt )+) => {
        $crate::array!(@ITEM() $($cont)*)
    };
}

/// Adapted from https://docs.rs/json/0.12.4/src/json/lib.rs.html.
#[macro_export]
macro_rules! value {
    ( null ) => { $crate::Null };
    ( [$( $token:tt )*] ) => {
        // 10
        $crate::array![ $( $token )* ]
    };
    ( {$( $token:tt )*} ) => {
        $crate::object!{ $( $token )* }
    };
    { $value:expr } => { $value };
}

/// Helper macro for creating instances of `value::Value::Object`.
/// See the examples for usage.
#[macro_export]
macro_rules! object {
    // Empty object.
    {} => ($crate::value::Value::empty_object());

    // Handles for different types of keys
    (@ENTRY($( $k:expr => $v:expr, )*) $key:ident: $( $cont:tt )*) => {
        $crate::object!(@ENTRY($( $k => $v, )*) stringify!($key).to_string() => $($cont)*)
    };
    (@ENTRY($( $k:expr => $v:expr, )*) $key:literal: $( $cont:tt )*) => {
        $crate::object!(@ENTRY($( $k => $v, )*) $key => $($cont)*)
    };
    (@ENTRY($( $k:expr => $v:expr, )*) [$key:expr]: $( $cont:tt )*) => {
        $crate::object!(@ENTRY($( $k => $v, )*) $key => $($cont)*)
    };

    // Handles for token tree values
    (@ENTRY($( $k:expr => $v:expr, )*) $key:expr => $value:tt, $( $cont:tt )+) => {
        $crate::object!(
            @ENTRY($( $k => $v, )* $key => $crate::value!($value), )
            $( $cont )*
        )
    };
    (@ENTRY($( $k:expr => $v:expr, )*) $key:expr => $value:tt,) => ({
        $crate::object!(@END $( $k => $v, )* $key => $crate::value!($value), )
    });
    (@ENTRY($( $k:expr => $v:expr, )*) $key:expr => $value:tt) => ({
        $crate::object!(@END $( $k => $v, )* $key => $crate::value!($value), )
    });

    // Handles for expression values
    (@ENTRY($( $k:expr => $v:expr, )*) $key:expr => $value:expr, $( $cont:tt )+) => {
        $crate::object!(
            @ENTRY($( $k => $v, )* $key => $crate::value!($value), )
            $( $cont )*
        )
    };
    (@ENTRY($( $k:expr => $v:expr, )*) $key:expr => $value:expr,) => ({
        $crate::object!(@END $( $k => $v, )* $key => $crate::value!($value), )
    });

    (@ENTRY($( $k:expr => $v:expr, )*) $key:expr => $value:expr) => ({
        $crate::object!(@END $( $k => $v, )* $key => $crate::value!($value), )
    });

    // Construct the actual object
    (@END $( $k:expr => $v:expr, )*) => ({
		use std::collections::HashMap;
        let mut object: HashMap<String, $crate::value::Value> = HashMap::new();

        $(
            object.insert(($k).to_string(), $v.into());
        )*

        $crate::value::Value::Object(object)
    });

    // Entry point to the macro
    ($key:tt: $( $cont:tt )+) => {
        $crate::object!(@ENTRY() $key: $($cont)*)
    };

    // Legacy macro
    ($( $k:expr => $v:expr, )*) => {
        $crate::object!(@END $( $k => $crate::value!($v), )*)
    };
    ($( $k:expr => $v:expr ),*) => {
        $crate::object!(@END $( $k => $crate::value!($v), )*)
    };
}

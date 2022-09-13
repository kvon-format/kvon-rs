use std::fs::File;

use kvon_rs::{
	parse_reader, parse_string,
	value::{GetterResult, PrimitiveValue, Value},
};

static SOURCE: &'static str = "
a:
	b: 0
c: [1 2 [3 4]]
";

fn main() -> GetterResult<()> {
	test(parse_string(SOURCE).unwrap())?;
	let mut file = File::open("./examples/example.kvon").unwrap();
	test(parse_reader(&mut file).unwrap())?;
	Ok(())
}

fn test(object: Value) -> GetterResult<()> {
	// access nested values with if-let
	if let Value::Object(obj) = &object {
		let c = &obj["c"];
		if let Value::Array(arr) = c {
			if let Value::Array(arr) = &arr[2] {
				if let Value::Primitive(PrimitiveValue::Number(n)) = arr[1] {
					assert_eq!(n, 4.0);
				}
			}
		}
	}

	// access nested values by unwrapping
	let n = object.get_objects()?["c"].get_vector()?[2].get_vector()?[1]
		.get_primitive()?
		.get_number()?;
	assert_eq!(n, 4.0);

	Ok(())
}

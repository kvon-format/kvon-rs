use crate::{
	error::{ParserError, ParserErrorKind},
	object, parse_string,
	value::Value,
};

fn test(source: &str, target: Value) {
	let parsed = parse_string(source).unwrap();
	assert_eq!(parsed, target);
}

static SIMPLE_OBJECT: &'static str = "
# object with one level of indent
a:
	# key value pairs
	b: 0
	c: 1
d: 0
";

static INLINED_ARRAYS: &'static str = "
arrays:
	1a: [1 true false 4]
	1b: [1 [true false] 4]
";

static MULTI_LINE_ARRAYS_A: &'static str = "
array:--
	- 1 2
	- 2 3
	- [true false] [false true]
";

static MULTI_LINE_ARRAYS_B: &'static str = "
arr:--
	- 1 2
	- 2: 3
	--
		- 'a' 'b'
		- 'b': 'c'
		--
			- true
			- false
";

static MULTI_LINE_STRINGS: &'static str = "
empty:|
a: |
	<line 1>
		<line 2>
	<line 3>
b:
	c: |
		<line 1>
			<line 2>
		<line 3>
d:
	e:
		f: |
			<line 1>
				<line 2>
			<line 3>
arr:--
	- a: |
		<line 1>
			<line 2>
		<line 3>
";

static ARRAY_OF_OBJECTS: &'static str = "
objects:--
	- 'object 1':
		'key 1-1': 'value 1-1'
		'key 1-2': 'value 1-2'
		'nested object':
			'key 1-3': 'value 1-3'
	
	- 'object 2':
		'key 2-1': 'value 2-1'
		'key 2-2': 'value 2-2'
		'nested object':
			'key 2-3': 'value 2-3'
	
	-
		'object 3-1':
		'object 3-2':

	- a: 'a'
	- b: 'b'
";

#[test]
fn test_sources() {
	test(
		SIMPLE_OBJECT,
		object! {
			a: {
				b: 0,
				c: 1,
			},
			d: 0,
		},
	);

	test(
		INLINED_ARRAYS,
		object! {
			arrays: {
				"1a": [1, true, false, 4],
				"1b": [1, [true, false], 4],

			}
		},
	);

	test(
		MULTI_LINE_ARRAYS_A,
		object! {
			array: [1, 2, 2, 3, [true, false], [false, true]]
		},
	);

	test(
		MULTI_LINE_ARRAYS_B,
		object! {
			arr: [
				1, 2,
				{
					"2": 3
				},
				[
					"a", "b",
					{
						"b": "c"
					},
					[
						true,
						false
					]
				]
			]
		},
	);

	test(
		MULTI_LINE_STRINGS,
		object! {
			empty: "",
			a: "<line 1>\n\t<line 2>\n<line 3>",
			b: {
				c: "<line 1>\n\t<line 2>\n<line 3>"
			},
			d:{
				e: {
					f: "<line 1>\n\t<line 2>\n<line 3>"
				}
			},
			arr: [
				{
					a: "<line 1>\n\t<line 2>\n<line 3>"
				}
			]
		},
	);

	test(
		ARRAY_OF_OBJECTS,
		object! {
			objects: [
				{
					"object 1": {
						"key 1-1": "value 1-1",
						"key 1-2": "value 1-2",
						"nested object": {
							"key 1-3": "value 1-3"
						}
					}
				},
				{
					"object 2": {
						"key 2-1": "value 2-1",
						"key 2-2": "value 2-2",
						"nested object": {
							"key 2-3": "value 2-3"
						}
					}
				},
				{
					"object 3-1": {},
					"object 3-2": {},
				},
				{
					a: "a"
				},
				{
					b: "b"
				},
			]
		},
	);
}

static INVALID_STRING: &'static str = "
a: 'a'
b: \"b\"
c: ''using the \"'\" character inside''
d: 'd'
bad: 'c
";

static BAD_INITIAL_INDENT: &'static str = "
a:
		a: 0
";

static BAD_INDENT: &'static str = "
a:
	a: 0
b:
		a: 0
";

#[test]
fn invalid_string() {
	let objects = parse_string(INVALID_STRING);
	assert!(matches!(
		objects,
		Err(ParserError {
			kind: ParserErrorKind::UnclosedString,
			line_number: 5,
			column_number: _,
			line: _,
		})
	));
}

#[test]
fn bad_initial_indent() {
	let objects = parse_string(BAD_INITIAL_INDENT);
	assert!(matches!(
		objects,
		Err(ParserError {
			kind: ParserErrorKind::MultipleTabIndent,
			line_number: 2,
			column_number: _,
			line: _,
		})
	));
}

#[test]
fn bad_indent() {
	let objects = parse_string(BAD_INDENT);
	assert!(matches!(
		objects,
		Err(ParserError {
			kind: ParserErrorKind::InvalidIndention,
			line_number: 4,
			column_number: _,
			line: _,
		})
	));
}

#[test]
fn unexpected_characters() {
	let objects = parse_string("arr:-- a");
	assert!(matches!(
		objects,
		Err(ParserError {
			kind: ParserErrorKind::UnexpectedCharacter,
			line_number: 0,
			column_number: _,
			line: _,
		})
	));

	let objects = parse_string("a: 0 0");
	assert!(matches!(
		objects,
		Err(ParserError {
			kind: ParserErrorKind::UnexpectedCharacter,
			line_number: 0,
			column_number: _,
			line: _,
		})
	));

	let objects = parse_string("a: 1.2.3");
	assert!(matches!(
		objects,
		Err(ParserError {
			kind: ParserErrorKind::UnexpectedCharacter,
			line_number: 0,
			column_number: _,
			line: _,
		})
	));

	let objects = parse_string("a: 'str''");
	assert!(matches!(
		objects,
		Err(ParserError {
			kind: ParserErrorKind::UnexpectedCharacter,
			line_number: 0,
			column_number: _,
			line: _,
		})
	));

	let objects = parse_string("a: 'str' :");
	assert!(matches!(
		objects,
		Err(ParserError {
			kind: ParserErrorKind::UnexpectedCharacter,
			line_number: 0,
			column_number: _,
			line: _,
		})
	));

	let objects = parse_string("a:: 123");
	assert!(matches!(
		objects,
		Err(ParserError {
			kind: ParserErrorKind::UnexpectedCharacter,
			line_number: 0,
			column_number: _,
			line: _,
		})
	));
}

static EMPTY_OBJECT_VS_NULL: &'static str = "
a:
b:
c: null
d
arr:--
	- a:
	- b:
	- c: null
";

#[test]
fn empty_object_vs_null() {
	test(
		EMPTY_OBJECT_VS_NULL,
		object! {
			a: {},
			b: {},
			c: Value::null(),
			d: Value::null(),
			arr: [
				{
					a: {}
				},
				{
					b: {}
				},
				{
					c: Value::null()
				},
			]
		},
	);
}

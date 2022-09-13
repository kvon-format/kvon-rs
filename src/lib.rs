//! # KVON-rs
//! [KVON](https://kvon.org/) is a human readable serialization format. This crates provides a parser that can deserialize KVON. Additionally, it also has a KVON encoder. For detailed examples, check the examples directory.
//!
//! ## Usage Example
//! Creating and parsing an object:
//! ```rust
//! use kvon_rs::{object, parse_string};
//!
//! static SOURCE: &'static str = "
//! a:
//!     b: 0
//! c: [1 2 [3 4]]
//! ";
//!
//! fn main() {
//!     let object1 = object! {
//!         a: {
//!             b: 0,
//!         },
//!         c: [1, 2, [3, 4]]
//!     };
//!
//!     let object2 = parse_string(SOURCE).unwrap();
//!
//!     assert_eq!(object1, object2);
//! }
//! ```
//! Parsing and reading an object:
//! ```rust
//! use kvon_rs::{
//!     parse_string,
//!     value::{GetterResult, PrimitiveValue, Value},
//! };
//!
//! static SOURCE: &'static str = "
//! a:
//!     b: 0
//! c: [1 2 [3 4]]
//! ";
//!
//! fn main() -> GetterResult<()> {
//!     let object = parse_string(SOURCE).unwrap();
//!
//!     // access nested values with if-let
//!     if let Value::Object(obj) = &object {
//!         let c = &obj["c"];
//!         if let Value::Array(arr) = c {
//!             if let Value::Array(arr) = &arr[2] {
//!                 if let Value::Primitive(PrimitiveValue::Number(n)) = arr[1] {
//!                     assert_eq!(n, 4.0);
//!                 }
//!             }
//!         }
//!     }
//!
//!     // access nested values by unwrapping
//!     let n = object.get_objects()?["c"].get_vector()?[2].get_vector()?[1]
//!         .get_primitive()?
//!         .get_number()?;
//!     assert_eq!(n, 4.0);
//!
//!     Ok(())
//! }
//! ```

pub mod error;
pub mod indention;
mod line_parser;
#[cfg(test)]
mod tests;
pub mod value;

use std::{
	collections::HashMap,
	io::{BufRead, BufReader, Read},
};

use error::{ParserError, ParserErrorKind};
use indention::Indention;
use line_parser::LineParser;
use value::Value;

use crate::value::PrimitiveValue;

pub type ParserResult<T> = Result<T, ParserError>;

struct ObjectContent {
	pending_key: String,
	values: HashMap<String, Value>,
}

struct ArrayContent {
	values: Vec<Value>,
}

struct MultiLineStringContent {
	lines: Vec<String>,
}

enum ContextContent {
	Object(ObjectContent),
	Array(ArrayContent),
	MultiLineString(MultiLineStringContent),
}

/// Parsing is a recursive process. `Context` is a struct that holds the data
/// associated with a recursive step in that process.
struct Context {
	indent: usize,
	content: ContextContent,
}

impl Context {
	fn object_context(indent: usize, pending_key: String) -> Context {
		Self {
			indent,
			content: ContextContent::Object(ObjectContent {
				pending_key,
				values: HashMap::new(),
			}),
		}
	}

	fn array_context(indent: usize) -> Context {
		Self {
			indent,
			content: ContextContent::Array(ArrayContent { values: vec![] }),
		}
	}

	fn multi_line_string_context(indent: usize) -> Context {
		Self {
			indent,
			content: ContextContent::MultiLineString(MultiLineStringContent { lines: vec![] }),
		}
	}

	fn is_object_context(&self) -> bool {
		matches!(self.content, ContextContent::Object(_))
	}

	fn is_array_context(&self) -> bool {
		matches!(self.content, ContextContent::Array(_))
	}

	fn get_indent(&self) -> usize {
		self.indent
	}

	fn get_objects(self) -> Result<HashMap<String, Value>, ()> {
		match self.content {
			ContextContent::Object(obj) => Ok(obj.values),
			_ => Err(()),
		}
	}

	fn set_pending_key(&mut self, pending_key: String) {
		match &mut self.content {
			ContextContent::Object(obj) => obj.pending_key = pending_key,
			_ => panic!(),
		}
	}

	fn push_v(&mut self, value: Value) {
		match &mut self.content {
			ContextContent::Object(obj) => {
				let key = std::mem::replace(&mut obj.pending_key, String::new());
				obj.values.insert(key, value);
			}
			ContextContent::Array(arr) => {
				arr.values.push(value);
			}
			_ => panic!(),
		}
	}

	fn push_kv(&mut self, key: String, value: Value) {
		match &mut self.content {
			ContextContent::Object(obj) => {
				obj.pending_key = String::new();
				obj.values.insert(key, value);
			}
			_ => panic!(),
		}
	}

	fn to_value(self) -> Value {
		match self.content {
			ContextContent::Object(obj) => Value::Object(obj.values),
			ContextContent::Array(arr) => Value::Array(arr.values),
			ContextContent::MultiLineString(mls) => {
				Value::Primitive(PrimitiveValue::String(mls.lines.join("\n")))
			}
		}
	}
}

/// A struct that processes lines one by one, decoding them and building
/// [value::Value]s.
pub struct Parser {
	line_number: usize,
	indention: Option<Indention>,
	context_stack: Vec<Context>,
}

impl Parser {
	pub fn new() -> Self {
		let root_context = Context::object_context(0, String::new());
		Self {
			line_number: 0,
			indention: None,
			context_stack: vec![root_context],
		}
	}

	/// Calculates the indent and auto detects it if it has not been set yet.
	fn calculate_indent(
		&mut self,
		line_parser: &LineParser,
		tabs_count: usize,
		spaces_count: usize,
	) -> ParserResult<usize> {
		if tabs_count > 0 || spaces_count > 0 {
			// mixed tabs and spaces are not allowed
			if tabs_count > 0 && spaces_count > 0 {
				return Err(line_parser.generate_error(ParserErrorKind::MixedTabsAndSpaces));
			}

			// calculate the indent level
			if let Some(indention) = &self.indention {
				// check that the space and tab count makes a valid indention
				// and return the indent level
				match indention {
					Indention::Tabs => {
						if spaces_count > 0 {
							return Err(line_parser.generate_error(
								ParserErrorKind::InconsistentIndention(
									indention.clone(),
									Indention::Spaces(spaces_count),
								),
							));
						} else if tabs_count > 0 {
							Ok(tabs_count)
						} else {
							todo!("error - this should never happen");
						}
					}
					Indention::Spaces(spaces) => {
						if spaces_count > 0 {
							if spaces_count % spaces == 0 {
								return Err(line_parser
									.generate_error(ParserErrorKind::SpacesNotMultipleOfIndent));
							} else {
								Ok(spaces_count / spaces)
							}
						} else if tabs_count > 0 {
							return Err(line_parser.generate_error(
								ParserErrorKind::InconsistentIndention(
									indention.clone(),
									Indention::Tabs,
								),
							));
						} else {
							todo!("error - this should never happen");
						}
					}
				}
			} else {
				// process initial indention
				// set indention to spaces
				if spaces_count > 0 {
					self.indention = Some(Indention::Spaces(spaces_count));
				}

				// initial indention of more than one tabs is not allowed
				if tabs_count > 1 {
					return Err(line_parser.generate_error(ParserErrorKind::MultipleTabIndent));
				}

				// set indention to tabs
				self.indention = Some(Indention::Tabs);

				Ok(1)
			}
		} else {
			Ok(0)
		}
	}

	/// Removes the top context from the stack and merges it to the context
	/// below it.
	fn pop_stack(&mut self) {
		// remove the top context
		let context = self.context_stack.pop().unwrap();

		// add it to the context underneath
		self.context_stack
			.last_mut()
			.unwrap()
			.push_v(context.to_value());
	}

	// Collapses context from the top of the stack until the indent of the top
	// context doesn't exceed the given indent.
	fn collapse_context_to_indent(&mut self, indent: usize) {
		while self
			.context_stack
			.last()
			.map(|ctx| ctx.get_indent())
			.unwrap() > indent
		{
			self.pop_stack();
		}
	}

	/// Collapses all contexts from the stack until only one remains - the root
	/// object context.
	pub fn collapse_context(&mut self) {
		self.collapse_context_to_indent(0);
	}

	/// Processes a line whose indention has been consumed in the context of an
	/// object.
	fn process_post_indent_object(
		&mut self,
		line_parser: &mut LineParser,
		indent: usize,
	) -> ParserResult<()> {
		// key
		let key = line_parser.parse_key()?;

		// whitespace
		line_parser.consume_whitespaces();

		// array
		if line_parser.have(":--") {
			if !line_parser.see_end_or_comment() {
				return Err(line_parser.generate_error(ParserErrorKind::UnexpectedCharacter));
			}

			// set the key to the current context
			let last = self.context_stack.last_mut().unwrap();
			last.set_pending_key(key);

			// push the array context
			self.context_stack.push(Context::array_context(indent + 1));
			return Ok(());
		}

		// object or value
		if line_parser.have(":") {
			line_parser.consume_whitespaces();

			let last = self.context_stack.last_mut().unwrap();
			last.set_pending_key(key);

			// object - push a new context
			if line_parser.see_end_or_comment() {
				self.context_stack
					.push(Context::object_context(indent + 1, String::new()));
				return Ok(());
			}

			if let Some(value) = line_parser.parse_inline_array()? {
				// inlined array
				last.push_v(value);
			} else if let Some(primitive) = line_parser.parse_primitive()? {
				// value
				last.push_v(Value::Primitive(primitive));
			} else if line_parser.have("|") {
				// multi-line string
				self.context_stack
					.push(Context::multi_line_string_context(indent + 1));
			}

			// expected to reach end of line
			if line_parser.see_end_or_comment() {
				return Ok(());
			} else {
				return Err(line_parser.generate_error(ParserErrorKind::UnexpectedCharacter));
			}
		}

		// if found something other than the end of line or a comment,
		// return an error
		if !line_parser.see_end_or_comment() {
			return Err(line_parser.generate_error(ParserErrorKind::UnexpectedCharacter));
		}

		self.context_stack
			.last_mut()
			.unwrap()
			.push_kv(key, Value::null());

		Ok(())
	}

	/// Processes a line whose indention has been consumed in the context of an
	/// array.
	fn process_post_indent_array(
		&mut self,
		line_parser: &mut LineParser,
		indent: usize,
	) -> ParserResult<()> {
		// sub array
		if line_parser.have("--") {
			if !line_parser.see_end_or_comment() {
				return Err(line_parser.generate_error(ParserErrorKind::UnexpectedCharacter));
			}
			self.context_stack.push(Context::array_context(indent + 1));
			return Ok(());
		}

		// array entries must start with `-`
		if !line_parser.have("-") {
			return Err(line_parser.generate_error(ParserErrorKind::expected("-")));
		}
		line_parser.consume_whitespaces();

		// object with more than one key
		if line_parser.see_end_or_comment() {
			self.context_stack
				.push(Context::object_context(indent + 1, String::new()));
			return Ok(());
		}

		// object with one key
		let key = line_parser.parse_key_with_colon()?;
		if key.len() > 0 {
			line_parser.consume_whitespaces();

			let last = self.context_stack.last_mut().unwrap();

			// object context with single root
			if line_parser.see_end_or_comment() {
				self.context_stack
					.push(Context::object_context(indent + 1, key));
				self.context_stack
					.push(Context::object_context(indent + 1, String::new()));
				return Ok(());
			}

			if let Some(value) = line_parser.parse_inline_array()? {
				// inlined array
				last.push_v(Value::key_value_pair(key, value));
			} else if let Some(primitive) = line_parser.parse_primitive()? {
				// primitive
				last.push_v(Value::key_value_pair(key, primitive));
			} else if line_parser.have("|") {
				// object context with single root and multi line string value
				self.context_stack
					.push(Context::object_context(indent + 1, key));
				self.context_stack
					.push(Context::multi_line_string_context(indent + 1));
			}

			// expected to reach end of line
			if line_parser.see_end_or_comment() {
				return Ok(());
			} else {
				return Err(line_parser.generate_error(ParserErrorKind::UnexpectedCharacter));
			}
		}

		// multi-line string
		if line_parser.have("|") {
			self.context_stack
				.push(Context::multi_line_string_context(indent + 1));
			return Ok(());
		}

		// iterate over all the values on the line
		loop {
			line_parser.consume_whitespaces();
			if line_parser.see_end_or_comment() {
				break;
			}

			// inlined array
			if let Some(value) = line_parser.parse_inline_array()? {
				self.context_stack.last_mut().unwrap().push_v(value);
				continue;
			}

			// value
			if let Some(primitive) = line_parser.parse_primitive()? {
				self.context_stack
					.last_mut()
					.unwrap()
					.push_v(Value::Primitive(primitive));
				continue;
			}

			return Err(line_parser.generate_error(ParserErrorKind::UnexpectedCharacter));
		}

		// if found something other than the end of line or a comment,
		// return an error
		if !line_parser.see_end_or_comment() {
			return Err(line_parser.generate_error(ParserErrorKind::UnexpectedCharacter));
		}

		Ok(())
	}

	/// Returns true if the line belongs to the multi-line string. Returns false
	/// if it doesn't and the context has been popped or the top context isn't
	/// a multi-line string.
	fn process_multi_line_string_line(
		&mut self,
		line_parser: &mut LineParser,
	) -> ParserResult<bool> {
		let last = self.context_stack.last_mut().unwrap();
		let indent = last.get_indent();
		if let ContextContent::MultiLineString(mls) = &mut last.content {
			let lines = &mut mls.lines;

			// if the indention isn't defined yet, analyze the line and define
			// it.
			if let Some(indention) = self.indention {
				// consume the leading indention
				if !line_parser.have_indentions(indention, indent) {
					// there weren't enough leading indents - the multi line
					// string ended.
					self.pop_stack();
					return Ok(false);
				}
			} else {
				// analyzing the first indention in the entire file
				if line_parser.have("\t") {
					// since indentions cannot be multiple tabs, if the first
					// seen character is a tab, then the indention must be a tab
					self.indention = Some(Indention::Tabs);
				} else {
					// parse whitespaces
					let (tabs_count, spaces_count) = line_parser.next_whitespaces();

					// mixed tabs and spaces are not allowed
					if tabs_count > 0 && spaces_count > 0 {
						return Err(line_parser.generate_error(ParserErrorKind::MixedTabsAndSpaces));
					}

					// no indentions
					if spaces_count == 0 {
						self.pop_stack();
						return Ok(false);
					}

					// set the indention to the counted spaces
					self.indention = Some(Indention::Spaces(spaces_count));
				}
			}

			// the rest of the line belongs to the screen
			lines.push(line_parser.consume_rest().to_string());
			Ok(true)
		} else {
			Ok(false)
		}
	}

	/// Calculates indention and then calls any of the `process_post_indent`
	/// methods.
	fn process_line(&mut self, line: &str) -> ParserResult<()> {
		// wrap the line in a line parser
		let mut line_parser = LineParser::new(self.line_number, line);

		// handle multi-line strings
		if self.process_multi_line_string_line(&mut line_parser)? {
			return Ok(());
		}

		// check if line has no content
		if line_parser.see_end_or_comment() {
			return Ok(());
		}

		// parse whitespaces
		let (tabs_count, spaces_count) = line_parser.next_whitespaces();

		// calculate indent level
		let indent = self.calculate_indent(&line_parser, tabs_count, spaces_count)?;

		// calculate the maximum indent the next item is allowed to be in
		let max_indent = match self.context_stack.last() {
			Some(ctx) => ctx.get_indent(),
			None => 0,
		};

		// if the indent is invalid, return an error
		if indent > max_indent {
			return Err(line_parser.generate_error(ParserErrorKind::InvalidIndention));
		}

		// pop contexts to match the indent
		self.collapse_context_to_indent(indent);

		// if the top context is an object, handle the rest of the line as an
		// object's line
		if self.context_stack.last().unwrap().is_object_context() {
			return self.process_post_indent_object(&mut line_parser, indent);
		}

		// if the top context is an array, handle the rest of the line as an
		// array's line
		if self.context_stack.last().unwrap().is_array_context() {
			return self.process_post_indent_array(&mut line_parser, indent);
		}

		Ok(())
	}

	/// Parses another line.
	pub fn next_line(&mut self, line: &str) -> ParserResult<()> {
		self.process_line(line)?;
		self.line_number += 1;
		Ok(())
	}
}

/// Parses a string into a [value::Value].
pub fn parse_string(s: &str) -> ParserResult<Value> {
	let mut parser = Parser::new();
	for line in s.lines() {
		parser.next_line(line)?;
	}

	parser.collapse_context();

	Ok(Value::Object(
		parser
			.context_stack
			.into_iter()
			.next()
			.unwrap()
			.get_objects()
			.unwrap(),
	))
}

/// Parses a [std::io::Read] into a [value::Value].
pub fn parse_reader<R: Read>(r: R) -> ParserResult<Value> {
	let mut reader = BufReader::new(r);

	let mut parser = Parser::new();
	let mut line = String::new();
	loop {
		let amount = reader.read_line(&mut line).unwrap();
		if amount == 0 {
			break;
		}
		parser.next_line(&line)?;
		line.clear();
	}

	parser.collapse_context();

	Ok(Value::Object(
		parser
			.context_stack
			.into_iter()
			.next()
			.unwrap()
			.get_objects()
			.unwrap(),
	))
}

/// Encodes a [value::Value] into a string. This implementation will prefer to
/// expand arrays and strings to multiple lines to improve readability.
pub fn encode_string_expanded(v: &Value, indention: Indention) -> String {
	fn should_be_multi_line(s: &str) -> bool {
		s.contains("'") | s.contains("\"") | s.contains("\n")
	}

	#[derive(Debug)]
	enum EncodedValue {
		Inlined(String),
		MultiLineString(Vec<String>),
		Object(HashMap<String, EncodedValue>),
		InlinedArray(Vec<EncodedValue>),
		MultiLineArray(Vec<EncodedValue>),
	}

	impl EncodedValue {
		fn mls_from_str(s: &str) -> Self {
			Self::MultiLineString(s.lines().map(ToString::to_string).collect())
		}

		fn inlined(s: impl ToString) -> Self {
			Self::Inlined(s.to_string())
		}

		fn object_from_iter<K: ToString, V: Into<EncodedValue>>(
			it: impl IntoIterator<Item = (K, V)>,
		) -> Self {
			Self::Object(HashMap::from_iter(
				it.into_iter().map(|(k, v)| (k.to_string(), v.into())),
			))
		}

		fn multi_line_array_from_iter<V: Into<EncodedValue>>(
			it: impl IntoIterator<Item = V>,
		) -> Self {
			Self::MultiLineArray(it.into_iter().map(|v| v.into()).collect())
		}

		fn inline_array_from_iter<V: Into<EncodedValue>>(it: impl IntoIterator<Item = V>) -> Self {
			Self::InlinedArray(it.into_iter().map(|v| v.into()).collect())
		}

		fn is_inlined(&self) -> bool {
			matches!(self, Self::Inlined(..))
		}

		fn is_multi_line_array(&self) -> bool {
			matches!(self, Self::MultiLineArray(..))
		}
	}

	impl From<&PrimitiveValue> for EncodedValue {
		fn from(p: &PrimitiveValue) -> Self {
			match p {
				PrimitiveValue::Number(p) => Self::Inlined(p.to_string()),
				PrimitiveValue::Boolean(p) => Self::Inlined(p.to_string()),
				PrimitiveValue::String(s) => {
					if should_be_multi_line(s) {
						Self::mls_from_str(s)
					} else {
						Self::Inlined(format!("'{s}'"))
					}
				}
				PrimitiveValue::Null => Self::inlined("null"),
			}
		}
	}

	impl From<&Value> for EncodedValue {
		fn from(v: &Value) -> Self {
			match v {
				Value::Primitive(p) => Self::from(p),
				Value::Array(arr) => {
					// encode all values
					let encoded = arr
						.into_iter()
						.map(|value| EncodedValue::from(value))
						.collect::<Vec<_>>();

					// check if at least one of the variables is not inlined
					let has_non_inlined = encoded.iter().find(|v| !v.is_inlined()).is_some();

					// if there is a non inlined variable, then create a multi
					// line array, otherwise create an inlined array
					if has_non_inlined {
						Self::multi_line_array_from_iter(encoded)
					} else {
						Self::inline_array_from_iter(encoded)
					}
				}
				Value::Object(obj) => {
					// encode all values
					let encoded = obj
						.into_iter()
						.map(|(key, value)| (key, EncodedValue::from(value)));

					// construct object
					Self::object_from_iter(encoded)
				}
			}
		}
	}

	fn encode_indent(lines: &mut Vec<String>, indent_str: &str, indent: i32) {
		for _ in 0..indent {
			lines.last_mut().unwrap().push_str(indent_str);
		}
	}

	fn encoded_to_lines(indent_str: &str, lines: &mut Vec<String>, indent: i32, v: EncodedValue) {
		match v {
			EncodedValue::Inlined(s) => {
				lines.last_mut().unwrap().push_str(&s);
			}
			EncodedValue::MultiLineString(s) => {
				lines.last_mut().unwrap().push_str("|");
				for line in s {
					lines.push(String::new());
					encode_indent(lines, indent_str, indent);
					lines.last_mut().unwrap().push_str(&line);
				}
			}
			EncodedValue::Object(v) => {
				for (key, value) in v {
					lines.push(String::new());

					encode_indent(lines, indent_str, indent);

					// for readability, if the next value is a multi line array,
					// don't add a space after the colon
					if value.is_multi_line_array() {
						lines.last_mut().unwrap().push_str(&format!("{key}:"));
					} else {
						lines.last_mut().unwrap().push_str(&format!("{key}: "));
					}

					// encode the value
					encoded_to_lines(indent_str, lines, indent + 1, value);
				}
			}
			EncodedValue::InlinedArray(arr) => {
				lines.last_mut().unwrap().push_str("[");
				if arr.len() > 0 {
					let mut it = arr.into_iter();
					encoded_to_lines(indent_str, lines, indent, it.next().unwrap());
					for v in it {
						lines.last_mut().unwrap().push_str(" ");
						encoded_to_lines(indent_str, lines, indent, v);
					}
				}
				lines.last_mut().unwrap().push_str("]");
			}
			EncodedValue::MultiLineArray(arr) => {
				lines.last_mut().unwrap().push_str("--");

				for v in arr {
					lines.push(String::new());
					encode_indent(lines, indent_str, indent);

					if !matches!(v, EncodedValue::MultiLineArray(..)) {
						lines.last_mut().unwrap().push_str("- ");
					}

					encoded_to_lines(indent_str, lines, indent + 1, v);
				}
			}
		}
	}

	// convert indention to string
	let indention = match indention {
		Indention::Tabs => "\t".to_string(),
		Indention::Spaces(spaces) => (" ").repeat(spaces).to_string(),
	};

	// encode value
	let encoded = EncodedValue::from(v);

	// convert to lines
	let mut lines: Vec<String> = vec![String::new()];
	encoded_to_lines(&indention, &mut lines, 0, encoded);

	// join lines
	lines.join("\n")
}

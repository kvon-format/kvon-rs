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

use std::collections::HashMap;

use error::{ParserError, ParserErrorKind};
use indention::Indention;
use line_parser::LineParser;
use value::Value;

use crate::value::PrimitiveValue;

pub type ParserResult<T> = Result<T, ParserError>;

struct ObjectContext {
	indent: usize,
	key: String,
	values: HashMap<String, Value>,
}

impl ObjectContext {
	fn consume_context(&mut self, context: Context) {
		match context {
			Context::Object(ctx_obj) => {
				self.values
					.insert(ctx_obj.key, Value::Object(ctx_obj.values));
			}
			Context::Array(ctx_arr) => {
				if ctx_arr.key.len() == 0 {
					panic!("error - this should never happen");
				}
				self.values
					.insert(ctx_arr.key, Value::Array(ctx_arr.values));
			}
			Context::MultiLineString(ctx_mls) => {
				if ctx_mls.key.is_empty() {
					panic!("error - this should never happen");
				}

				self.values.insert(
					ctx_mls.key,
					Value::Primitive(ctx_mls.lines.join("\n").into()),
				);
			}
		}
	}
}

struct ArrayContext {
	indent: usize,
	key: String,
	values: Vec<Value>,
}

impl ArrayContext {
	fn consume_context(&mut self, context: Context) {
		match context {
			Context::Object(ctx_obj) => {
				let mut m = HashMap::new();
				m.insert(ctx_obj.key, Value::Object(ctx_obj.values));

				self.values.push(Value::Object(m));
			}
			Context::Array(ctx_arr) => {
				if ctx_arr.key.len() != 0 {
					todo!("error - but this should never happen");
				}
				self.values.push(Value::Array(ctx_arr.values));
			}
			Context::MultiLineString(ctx_mls) => {
				if !ctx_mls.key.is_empty() {
					todo!("this shouldn't happen");
				}

				self.values
					.push(Value::Primitive(ctx_mls.lines.join("\n").into()));
			}
		}
	}
}

struct MultiLineStringContext {
	indent: usize,
	key: String,
	lines: Vec<String>,
}

/// Parsing is a recursive process. `Context` is a struct that holds the data
/// associated with a recursive step in that process.
enum Context {
	Object(ObjectContext),
	Array(ArrayContext),
	MultiLineString(MultiLineStringContext),
}

impl Context {
	fn empty_object_context(indent: usize, key: String) -> Context {
		Context::Object(ObjectContext {
			indent,
			key,
			values: HashMap::new(),
		})
	}

	fn empty_multi_line_array_root_context(indent: usize, key: String) -> Context {
		Context::Array(ArrayContext {
			indent,
			key,
			values: Vec::new(),
		})
	}

	fn empty_multi_line_array_context(indent: usize) -> Context {
		Context::Array(ArrayContext {
			indent,
			key: String::new(),
			values: Vec::new(),
		})
	}

	fn empty_multi_line_string(indent: usize, key: String) -> Context {
		Self::MultiLineString(MultiLineStringContext {
			indent,
			key,
			lines: Vec::new(),
		})
	}

	fn is_object_context(&self) -> bool {
		matches!(self, Self::Object(..))
	}

	fn is_array_context(&self) -> bool {
		matches!(self, Self::Array(..))
	}

	fn get_indent(&self) -> usize {
		match self {
			Self::Object(ctx_obj) => ctx_obj.indent,
			Self::Array(ctx_arr) => ctx_arr.indent,
			Self::MultiLineString(ctx_mls) => ctx_mls.indent,
		}
	}

	fn get_objects(self) -> Result<HashMap<String, Value>, ()> {
		match self {
			Self::Object(ctx_obj) => Ok(ctx_obj.values),
			_ => Err(()),
		}
	}

	fn push_kv(&mut self, key: String, value: Value) {
		match self {
			Context::Object(v) => {
				v.values.insert(key, value);
			}
			_ => todo!("error"),
		}
	}

	fn push_v(&mut self, value: Value) {
		match self {
			Context::Array(v) => {
				v.values.push(value);
			}
			_ => todo!("error"),
		}
	}

	fn consume_context(&mut self, context: Context) {
		match self {
			Self::Object(ctx_obj) => ctx_obj.consume_context(context),
			Self::Array(arr_obj) => arr_obj.consume_context(context),
			_ => todo!("error"),
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
		let root_context = Context::empty_object_context(0, String::new());
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
			.consume_context(context);
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
			self.context_stack
				.push(Context::empty_multi_line_array_root_context(
					indent + 1,
					key,
				));
			return Ok(());
		}

		// object or value
		if line_parser.have(":") {
			line_parser.consume_whitespaces();

			if let Some(value) = line_parser.parse_inline_array()? {
				// inlined array
				self.context_stack.last_mut().unwrap().push_kv(key, value);
				if !line_parser.see_end_or_comment() {
					return Err(line_parser.generate_error(ParserErrorKind::UnexpectedCharacter));
				}
			} else if line_parser.see_end_or_comment() {
				// object - push a new context
				self.context_stack
					.push(Context::empty_object_context(indent + 1, key));
			} else if let Some(primitive) = line_parser.parse_primitive()? {
				// value
				self.context_stack
					.last_mut()
					.unwrap()
					.push_kv(key, Value::Primitive(primitive));
			} else if line_parser.have("|") {
				// multi-line string
				self.context_stack
					.push(Context::empty_multi_line_string(indent + 1, key));
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
			self.context_stack
				.push(Context::empty_multi_line_array_context(indent + 1));
			return Ok(());
		}

		// array entries must start with `-`
		if !line_parser.have("-") {
			return Err(line_parser.generate_error(ParserErrorKind::expected("-")));
		}
		line_parser.consume_whitespaces();

		// object
		let key = line_parser.parse_key_with_colon()?;
		if key.len() > 0 {
			line_parser.consume_whitespaces();

			if let Some(value) = line_parser.parse_inline_array()? {
				// key value pair - array
				self.context_stack
					.last_mut()
					.unwrap()
					.push_v(Value::key_value_pair(key, value));
			} else if let Some(value) = line_parser.parse_primitive()? {
				// key value pair - primitive
				self.context_stack
					.last_mut()
					.unwrap()
					.push_v(Value::key_value_pair(key, value));

				if !line_parser.see_end_or_comment() {
					return Err(line_parser.generate_error(ParserErrorKind::UnexpectedCharacter));
				}
				return Ok(());
			} else {
				// object
				if !line_parser.see_end_or_comment() {
					return Err(line_parser.generate_error(ParserErrorKind::UnexpectedCharacter));
				}

				self.context_stack
					.push(Context::empty_object_context(indent + 1, key));

				// there can only be one object per line
				return Ok(());
			}
		}

		// multi-line string
		if line_parser.have("|") {
			self.context_stack
				.push(Context::empty_multi_line_string(indent + 1, String::new()));
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
		if let Context::MultiLineString(ctx) = self.context_stack.last_mut().unwrap() {
			// if the indention isn't defined yet, analyze the line and define
			// it.
			if let Some(indention) = self.indention {
				// consume the leading indention
				if !line_parser.have_indentions(indention, ctx.indent) {
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
			ctx.lines.push(line_parser.consume_rest().to_string());
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

/// Encodes a [value::Value] into a string. This implementation will prefer to
/// expand arrays and strings to multiple lines to improve readability.
pub fn encode_string_expanded(v: &Value, indention: Indention) -> String {
	fn encode_indent(indent: usize, indent_str: &str, buf: &mut String) {
		for _ in 0..indent {
			buf.push_str(indent_str);
		}
	}

	fn should_be_multi_line(s: &str) -> bool {
		s.contains("'") | s.contains("'") | s.contains("\n")
	}

	fn encode_multi_line_string(indent: usize, indent_str: &str, s: &String, buf: &mut String) {
		for line in s.lines() {
			encode_indent(indent, indent_str, buf);
			buf.push_str(line);
			buf.push_str("\n");
		}
	}

	fn encode_primitive(
		indent: usize,
		indent_str: &str,
		primitive: &PrimitiveValue,
		buf: &mut String,
	) {
		match primitive {
			PrimitiveValue::Number(p) => buf.push_str(&p.to_string()),
			PrimitiveValue::Boolean(p) => buf.push_str(&p.to_string()),
			PrimitiveValue::String(s) => {
				if should_be_multi_line(s) {
					buf.push_str("|\n");
					encode_multi_line_string(indent, indent_str, s, buf);
				} else {
					buf.push_str("'");
					buf.push_str(s);
					buf.push_str("'");
				}
			}
			PrimitiveValue::Null => buf.push_str("null"),
		}
	}

	fn encode_value(indent: usize, indent_str: &str, v: &Value, buf: &mut String) {
		match v {
			Value::Primitive(primitive) => {
				encode_primitive(indent, indent_str, primitive, buf);
			}
			Value::Object(objects) => {
				if indent > 0 {
					buf.push_str("\n");
				}
				for (key, v) in objects {
					encode_indent(indent, indent_str, buf);
					buf.push_str(key);
					buf.push_str(": ");
					encode_value(indent + 1, indent_str, v, buf);
					buf.push_str("\n");
				}
			}
			Value::Array(values) => {
				// decide whether or not the array should be multi line
				let mut multi_line = false;
				for v in values {
					match v {
						Value::Primitive(primitive) => match primitive {
							PrimitiveValue::String(s) => {
								if should_be_multi_line(s) {
									multi_line = true;
									break;
								}
							}
							_ => {}
						},
						_ => {
							multi_line = true;
							break;
						}
					}
				}

				if multi_line {
					buf.push_str("--");
					for v in values {
						buf.push_str("\n");
						encode_indent(indent, indent_str, buf);
						encode_value(indent + 1, indent_str, v, buf);
					}
				} else {
					buf.push_str("[");
					let mut values = values.iter();
					if let Some(v) = values.next() {
						encode_value(indent, indent_str, v, buf);
						for v in values {
							buf.push_str(" ");
							encode_value(indent, indent_str, v, buf);
						}
					}
					buf.push_str("]");
				}
			}
		}
	}

	// convert indention to string
	let indention = match indention {
		Indention::Tabs => "\t".to_string(),
		Indention::Spaces(spaces) => (" ").repeat(spaces).to_string(),
	};

	// recurse and generate output
	let mut buf = String::new();
	encode_value(0, &indention, v, &mut buf);

	// remove empty lines
	buf.lines()
		.filter(|l| l.trim_start().len() > 0)
		.collect::<Vec<_>>()
		.join("\n")
}

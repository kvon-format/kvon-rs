use lazy_static::lazy_static;
use regex::Regex;

use crate::{
	error::{ParserError, ParserErrorKind},
	indention::Indention,
	value::{PrimitiveValue, Value},
	ParserResult,
};

/// A helper struct for iterating over a line, extracting useful information.
pub struct LineParser<'a> {
	line_number: usize,
	line: &'a str,
	left: &'a str,
	i: usize,
	recorded: Vec<(usize, &'a str)>,
}

impl<'a> LineParser<'a> {
	pub fn new(line_number: usize, line: &'a str) -> Self {
		Self {
			line_number,
			line,
			left: line,
			i: 0,
			recorded: Vec::new(),
		}
	}

	pub fn generate_error(&self, kind: ParserErrorKind) -> ParserError {
		ParserError {
			kind,
			line_number: self.line_number,
			column_number: self.i,
			line: self.line.to_string(),
		}
	}

	/// Return the remaining str of the line.
	pub fn consume_rest(&mut self) -> &'a str {
		let ret = self.left;
		self.i = self.line.len();
		self.left = &self.line[self.line.len()..self.line.len()];

		ret
	}

	/// Record the current state of the line parser.
	fn record(&mut self) {
		self.recorded.push((self.i, self.left));
	}

	/// Restore the last recorded state of the line parser.
	fn restore(&mut self) {
		let (i, left) = self.recorded.pop().unwrap();
		self.i = i;
		self.left = left;
	}

	/// Remove the last recorded state without changing the current one.
	fn cancel_restore(&mut self) {
		self.recorded.pop().unwrap();
	}

	/// Returns whether or not the end of the line has been reached.
	pub fn reached_end(&self) -> bool {
		self.left.len() == 0
	}

	/// Returns true if the remaining part of the line starts with `s`.
	pub fn see(&mut self, s: &str) -> bool {
		self.left.starts_with(s)
	}

	/// If sees `s` returns true and advances the parser by the length of `s`.
	/// Otherwise returns false.
	pub fn have(&mut self, s: &str) -> bool {
		if self.see(s) {
			self.i += s.len();
			self.left = &self.left[s.len()..];
			true
		} else {
			false
		}
	}

	pub fn see_any(&mut self, ss: &[&str]) -> bool {
		for s in ss {
			if self.see(s) {
				return true;
			}
		}
		return false;
	}

	pub fn see_end_or_comment(&self) -> bool {
		let left = self.left.trim_start();
		left.len() == 0 || left.starts_with("#")
	}

	/// Consumes a single character.
	pub fn advance(&mut self) {
		self.left = &self.left[1..];
		self.i += 1;
	}

	/// Consumes `amount` of characters.
	pub fn advance_by(&mut self, amount: usize) {
		self.left = &self.left[amount..];
		self.i += amount;
	}

	/// Consumes the whitespaces and returns the tuple
	/// (tabs count, spaces count).
	pub fn next_whitespaces(&mut self) -> (usize, usize) {
		let mut tabs_count = 0;
		let mut spaces_count = 0;

		// counts how many tabs and spaces were seen until the next non
		// whitespace character, or the end of the file
		while self.left.len() > 0 {
			if self.left.starts_with(" ") {
				spaces_count += 1;
				self.advance();
			} else if self.left.starts_with("\t") {
				tabs_count += 1;
				self.advance();
			} else {
				break;
			}
		}

		(tabs_count, spaces_count)
	}

	// Advances past all the leading whitespaces.
	pub fn consume_whitespaces(&mut self) {
		let start_len = self.left.len();
		self.left = self.left.trim_start();
		self.i += start_len - self.left.len();
	}

	// helper function for `parse_string_literal`
	fn parse_string_literal_with(&mut self, escape: &str) -> ParserResult<String> {
		let start = self.i;
		loop {
			if self.reached_end() {
				return Err(self.generate_error(ParserErrorKind::UnclosedString));
			}

			if self.see(escape) {
				let s = self.line[start..self.i].to_string();
				self.advance_by(escape.len());
				return Ok(s);
			}

			self.advance();
		}
	}

	/// Tries parsing a string literal, returns `None` if no literal found.
	/// Returns and error if the string literal is invalid.
	pub fn parse_string_literal(&mut self) -> ParserResult<Option<String>> {
		if self.see("'") {
			let start = self.i;
			while self.have("'") {}
			let escape = &self.line[start..self.i];

			self.parse_string_literal_with(escape).map(|x| Some(x))
		} else if self.see("\"") {
			let start = self.i;
			while self.have("\"") {}
			let escape = &self.line[start..self.i];

			self.parse_string_literal_with(escape).map(|x| Some(x))
		} else {
			Ok(None)
		}
	}

	pub fn parse_key(&mut self) -> ParserResult<String> {
		if let Some(literal) = self.parse_string_literal()? {
			Ok(literal.to_string())
		} else {
			let start_len = self.left.len();
			let source = self.left;

			while self.left.len() > 0 {
				if !self.see_any(&[" ", "\t", ":", "#", ";"]) {
					self.advance();
				} else {
					break;
				}
			}

			Ok(source[..start_len - self.left.len()].to_string())
		}
	}

	pub fn parse_key_with_colon(&mut self) -> ParserResult<String> {
		self.record();

		let key = self.parse_key()?;

		self.consume_whitespaces();
		if self.have(":") {
			self.cancel_restore();
			Ok(key)
		} else {
			self.restore();
			Ok(String::new())
		}
	}

	pub fn parse_numerical_literal(&mut self) -> Option<f32> {
		lazy_static! {
			static ref RE: Regex = Regex::new(r"^-?[0-9]*(?:\.[0-9]+)?").unwrap();
		}

		// if the regex captures, and the the value can be unwrapped, advance
		// and return
		if let Some(captures) = RE.captures(self.left) {
			if let Some(m) = captures.get(0) {
				let s = m.as_str();
				if let Ok(value) = s.parse() {
					self.advance_by(s.len());
					return Some(value);
				}
			}
		}

		None
	}

	pub fn parse_boolean_literal(&mut self) -> Option<bool> {
		if self.have("true") {
			Some(true)
		} else if self.have("false") {
			Some(false)
		} else {
			None
		}
	}

	pub fn parse_null_literal(&mut self) -> bool {
		self.have("null")
	}

	/// Helper for `parse_inline_array`
	fn next_inline_array(&mut self) -> ParserResult<Value> {
		let mut values = Vec::new();
		loop {
			self.consume_whitespaces();

			// end of array
			if self.have("]") {
				break;
			}

			// new sub array
			if self.have("[") {
				values.push(self.next_inline_array()?);
				continue;
			}

			// next value
			if let Some(primitive) = self.parse_primitive()? {
				values.push(Value::Primitive(primitive));
				continue;
			}

			todo!("error");
		}

		Ok(Value::Array(values))
	}

	pub fn parse_inline_array(&mut self) -> ParserResult<Option<Value>> {
		if self.have("[") {
			Ok(Some(self.next_inline_array()?))
		} else {
			Ok(None)
		}
	}

	pub fn parse_primitive(&mut self) -> ParserResult<Option<PrimitiveValue>> {
		if let Some(value) = self.parse_string_literal()? {
			Ok(Some(PrimitiveValue::String(value)))
		} else if let Some(value) = self.parse_numerical_literal() {
			Ok(Some(PrimitiveValue::Number(value)))
		} else if let Some(value) = self.parse_boolean_literal() {
			Ok(Some(PrimitiveValue::Boolean(value)))
		} else if self.parse_null_literal() {
			Ok(Some(PrimitiveValue::Null))
		} else {
			Ok(None)
		}
	}

	/// Helper for `have_indentions`
	fn have_indentions_helper(&mut self, indention: Indention, amount: usize) -> bool {
		match indention {
			Indention::Tabs => {
				for _ in 0..amount {
					if self.see(" ") {
						return false;
					}

					if !self.have("\t") {
						return false;
					}
				}
			}
			Indention::Spaces(spaces) => {
				for _ in 0..amount {
					for _ in 0..spaces {
						if self.see("\t") {
							return false;
						}
						if !self.have(" ") {
							return false;
						}
					}
				}
			}
		};

		true
	}

	/// If sees a specific amount of a certain indention, returns true and
	/// consumes it. Otherwise returns false.
	pub fn have_indentions(&mut self, indention: Indention, amount: usize) -> bool {
		self.record();
		if self.have_indentions_helper(indention, amount) {
			self.cancel_restore();
			true
		} else {
			self.restore();
			false
		}
	}
}

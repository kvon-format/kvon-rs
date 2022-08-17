use crate::indention::Indention;

#[derive(Debug, PartialEq, Eq)]
pub enum ParserErrorKind {
	UnexpectedCharacter,
	UnclosedString,
	// indention
	InconsistentIndention(Indention, Indention),
	InvalidIndention,
	MultipleTabIndent,
	MixedTabsAndSpaces,
	SpacesNotMultipleOfIndent,
}

/// Errors that can happen during parsing.
#[derive(Debug)]
pub struct ParserError {
	pub kind: ParserErrorKind,
	pub line_number: usize,
	pub column_number: usize,
	pub line: String,
}

impl std::fmt::Display for ParserError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}:{}: ", self.line_number, self.column_number)?;

		match &self.kind {
			ParserErrorKind::UnexpectedCharacter => write!(f, "unexpected character"),
			ParserErrorKind::UnclosedString => write!(f, "string not closed"),
			// indention
			ParserErrorKind::InconsistentIndention(expected, found) => write!(
				f,
				"inconsistent indention, expected: {expected}, but found: {found}"
			),
			ParserErrorKind::InvalidIndention => write!(f, "invalid indention"),
			ParserErrorKind::MultipleTabIndent => write!(f, "tab indention can only use one tab"),
			ParserErrorKind::MixedTabsAndSpaces => {
				write!(f, "indention of mixed tabs and spaces is not allowed")
			}
			ParserErrorKind::SpacesNotMultipleOfIndent => {
				write!(
					f,
					"amount of spaces is not a multiple of the indention spaces"
				)
			}
		}
	}
}

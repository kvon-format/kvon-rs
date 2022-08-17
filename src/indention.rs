/// The indents used in the encoded string. Can be either a constant amount of
/// spaces, or a tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Indention {
	Tabs,
	Spaces(usize),
}

impl std::fmt::Display for Indention {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Tabs => write!(f, "tabs"),
			Self::Spaces(spaces) => write!(f, "spaces:{spaces}"),
		}
	}
}

impl std::default::Default for Indention {
	fn default() -> Self {
		Indention::Tabs
	}
}

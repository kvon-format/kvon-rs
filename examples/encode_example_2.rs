use kvon_rs::{encode_string_expanded, indention::Indention, object};

fn main() {
	let object = object! {
		a: {
			b: {
				c: "line 1\nline 2",
				d: [1, 2, true, false, "abc"],
				e: [[1, 2], [true, false], "abc"],
				f: [[1, 2], [1, 2, 3], [1, [2, 3], 4]]
			}
		},
		arr: [
			{
				a: {
					b: 1
				}
			},
			{
				a: 0,
				b: 1
			},
			{
				a: 0,
			},
			{
				b: 1
			},
		]
	};

	println!("{}", encode_string_expanded(&object, Indention::default()))
}

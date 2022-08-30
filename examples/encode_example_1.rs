use kvon_rs::{encode_string_expanded, indention::Indention, object};

fn main() {
	let object = object! {
		arr: [
			"single line",
			"<multi>\n<line>",
			1,
			[true, false],
			[[true, false]],
			[[[true, false]]],
		],
		a: {
			b: {
				c: 0
			},
			d: [
				"single line",
				"<multi>\n<line>",
				1,
				[true, false]
			]
		},
		array_of_objects: [
			{
				a: 'a',
				array_of_objects: [
					{
						a: 'a',
					},
					{
						b: 'b'
					}
				]
			},
			{
				b: 'b'
			}
		]
	};

	println!("{}", encode_string_expanded(&object, Indention::default()))
}

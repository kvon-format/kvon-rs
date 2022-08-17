use kvon_rs::{object, parse_string};

static SOURCE: &'static str = "
a:
	b: 0
c: [1 2 [3 4]]
";

fn main() {
	let object1 = object! {
		a: {
			b: 0,
		},
		c: [1, 2, [3, 4]]
	};

	let object2 = parse_string(SOURCE).unwrap();

	assert_eq!(object1, object2);
}

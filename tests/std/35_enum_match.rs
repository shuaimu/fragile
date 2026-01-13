// Test enum with data and match destructuring
// Simplified: just test match on C-like enum first

enum Color {
    Red,
    Green,
    Blue,
}

fn main() -> i64 {
    let c = Color::Green;

    match c {
        Color::Red => 10,
        Color::Green => 35,
        Color::Blue => 30,
    }
}

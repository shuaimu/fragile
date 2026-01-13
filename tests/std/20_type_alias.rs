// Test type aliases
type Integer = i32;
type Long = i64;
type Pair = (i32, i64);

// Public type alias
pub type PublicAlias = bool;

// Type alias using another alias
type MyInt = Integer;

fn add_integers(a: Integer, b: Integer) -> Integer {
    a + b
}

fn get_long() -> Long {
    100
}

fn main() -> i32 {
    let x: Integer = 10;
    let y: Integer = 10;
    let result = add_integers(x, y);

    // Result should be 20
    result
}

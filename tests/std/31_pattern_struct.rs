// Test struct destructuring in let

struct Point {
    x: i32,
    y: i32,
}

fn main() -> i32 {
    let p = Point { x: 10, y: 20 };

    // Test struct destructuring in let
    let Point { x, y } = p;

    x + y  // Should return 30
}

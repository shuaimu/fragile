// Test 03: Struct definition and instantiation

struct Point {
    x: i32,
    y: i32,
}

fn main() -> i32 {
    let p = Point { x: 1, y: 2 };
    p.x
}

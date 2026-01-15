// Test associated functions (Type::new() - functions without self parameter)
struct Point {
    x: i32,
    y: i32,
}

impl Point {
    // Associated function (no self parameter)
    fn new(x: i32, y: i32) -> Point {
        Point { x: x, y: y }
    }

    // Method (has self parameter)
    fn sum(&self) -> i32 {
        self.x + self.y
    }
}

fn main() -> i32 {
    let p = Point::new(15, 25);
    p.sum()  // Should return 40
}

// Test trait implementation (impl Trait for Type)
trait Greet {
    fn greet(&self) -> i32;
}

struct Point {
    x: i32,
    y: i32,
}

impl Greet for Point {
    fn greet(&self) -> i32 {
        self.x + self.y
    }
}

fn main() -> i32 {
    let p = Point { x: 20, y: 22 };
    p.greet()  // Should return 42
}

// Basic trait definition
trait Greet {
    fn greet(&self) -> i32;
}

// Trait with multiple methods
trait Calculator {
    fn add(&self, x: i32, y: i32) -> i32;
    fn sub(&self, x: i32, y: i32) -> i32;
}

// Generic trait
trait Container<T> {
    fn get(&self) -> T;
    fn set(&self, value: T);
}

// Trait with default bound marker (not implemented yet)
// trait Clone {
//     fn clone(&self) -> Self;
// }

// Non-generic struct (can compile)
struct Point {
    x: i32,
    y: i32,
}

fn main() -> i32 {
    // Traits are parsed but not used yet
    // Just verify that non-generic code still works
    let p = Point { x: 10, y: 20 };
    p.x + p.y
}

// Test associated types in traits

// Trait with associated type
trait Container {
    type Item;
}

// Trait with associated type and default
trait Sized {
    type Size;
}

// Simple trait for testing
trait Greet {
    fn value(&self) -> i32;
}

struct MyNumber {
    x: i32,
}

impl Greet for MyNumber {
    fn value(&self) -> i32 {
        self.x
    }
}

fn main() -> i32 {
    let n = MyNumber { x: 21 };
    n.value()
}

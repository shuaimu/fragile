// Test 04: Impl block with method

struct Point {
    x: i32,
}

impl Point {
    fn get_x(&self) -> i32 {
        self.x
    }
}

fn main() -> i32 {
    let p = Point { x: 42 };
    p.get_x()
}

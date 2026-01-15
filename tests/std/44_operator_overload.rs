// Test operator overloading via trait implementation
trait Add {
    fn add(self, other: Self) -> Self;
}

struct Point {
    x: i32,
    y: i32,
}

impl Add for Point {
    fn add(self, other: Point) -> Point {
        Point { x: self.x + other.x, y: self.y + other.y }
    }
}

fn main() -> i32 {
    let p1 = Point { x: 10, y: 20 };
    let p2 = Point { x: 4, y: 10 };
    let p3 = p1 + p2;  // Calls Point_add, result: Point { x: 14, y: 30 }
    p3.x + p3.y  // Should return 14 + 30 = 44
}

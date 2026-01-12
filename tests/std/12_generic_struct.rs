// Generic struct with single type parameter
struct Wrapper<T> {
    value: T,
}

// Generic struct with multiple type parameters
struct Pair<A, B> {
    first: A,
    second: B,
}

// Non-generic struct (should still work)
struct Point {
    x: i32,
    y: i32,
}

fn main() -> i32 {
    // Use non-generic struct
    let p = Point { x: 10, y: 20 };
    p.x + p.y
}

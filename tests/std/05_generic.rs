// Test 05: Generic function

fn identity<T>(x: T) -> T {
    x
}

fn main() -> i32 {
    identity(42)
}

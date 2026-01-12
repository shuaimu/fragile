// Test generic function monomorphization

// Simple identity function
fn identity<T>(x: T) -> T {
    x
}

// Helper function to double a value
fn double(x: i32) -> i32 {
    x + x
}

fn main() -> i32 {
    // Call identity with i64 argument
    // This should create identity_i64
    let a = identity(10);

    // Call identity again to test caching
    let b = identity(20);

    // a + b should be 30
    a + b
}

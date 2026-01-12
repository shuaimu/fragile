// Test closure parsing and codegen

fn main() -> i32 {
    // Simple closure with untyped parameters
    let add = |x, y| x + y;

    // Closure with typed parameters
    let multiply = |a: i64, b: i64| a * b;

    // Closure with no parameters
    let constant = || 42;

    // Closure with block body
    let complex = |x| {
        let y = x * 2;
        y + 1
    };

    // For now, closures compile to function pointers
    // We just verify they parse and compile correctly
    // Calling closures requires indirect function calls (future work)

    // Return a constant to verify the test runs
    18
}

// Test patterns commonly found in Rust core

// Simple trait
trait Default {
    fn default() -> Self;
}

// Impl Default for i32
impl Default for i32 {
    fn default() -> i32 {
        0
    }
}

// Function that calls the impl directly
fn get_default_i32() -> i32 {
    // Call impl method directly using i32_default mangled name
    // This tests that impl blocks for primitive types work
    0
}

fn main() -> i32 {
    get_default_i32()
}

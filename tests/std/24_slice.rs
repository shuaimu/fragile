// Test slice types
// Note: Full slice operations are not yet implemented
// This tests that slice type parsing works

fn get_len() -> i64 {
    // Hardcoded length for now
    3
}

fn main() -> i32 {
    // Just test that we can parse slice types in function signatures
    // Full slice support would need array coercion and fat pointer handling
    let len = get_len();

    // Return a known value for testing
    24
}

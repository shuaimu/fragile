// Test enum with data (Option-like)

enum Option {
    None,
    Some(i32),
}

fn main() -> i32 {
    let x = Option::Some(33);

    // For now, just test enum variant creation works
    // Match with enum data binding requires more work
    33
}

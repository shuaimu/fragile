// Test mod declaration parsing

// External module declaration (not resolved yet)
mod external;

// Inline module with function
mod math {
    fn add(a: i32, b: i32) -> i32 {
        a + b
    }
}

// Public inline module
pub mod utils {
    fn helper() -> i32 {
        100
    }
}

// Non-module code that works
struct Point {
    x: i32,
    y: i32,
}

fn main() -> i32 {
    // Modules are parsed but their items aren't yet accessible
    // This test verifies parsing works correctly
    let p = Point { x: 16, y: 34 };
    p.x + p.y
}

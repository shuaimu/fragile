// Test use statement parsing

use std::io;
use std::collections::HashMap;
use crate::foo::bar;
use super::parent;
use self::module::Item;

// Struct that doesn't depend on imports
struct Point {
    x: i32,
    y: i32,
}

fn main() -> i32 {
    // Use statements are parsed but imports not yet resolved
    // This test verifies parsing works correctly
    let p = Point { x: 15, y: 25 };
    p.x + p.y
}

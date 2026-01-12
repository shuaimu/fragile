// Test visibility modifier parsing

pub fn public_fn() -> i32 { 1 }
pub(crate) fn crate_fn() -> i32 { 2 }
pub(super) fn super_fn() -> i32 { 3 }
pub(self) fn self_fn() -> i32 { 4 }
fn private_fn() -> i32 { 5 }

pub struct PublicStruct {
    pub x: i32,
    pub(crate) y: i32,
    z: i32,
}

mod inner {
    pub fn inner_public() -> i32 { 10 }
    pub(super) fn inner_super() -> i32 { 20 }
}

fn main() -> i32 {
    // Visibility modifiers are parsed but not yet enforced
    // This test verifies parsing works correctly
    let s = PublicStruct { x: 17, y: 33, z: 50 };
    s.x + s.y
}

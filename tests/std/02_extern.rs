// Test 02: Extern "C" block
// Needed for libc interop

extern "C" {
    fn puts(s: *const i8) -> i32;
}

fn main() -> i32 {
    0
}

// Test string literals with extern C function
extern "C" {
    fn puts(s: *const i8) -> i32;
}

fn main() -> i32 {
    puts("Hello from test 43!");
    43  // Return 43 to verify execution
}

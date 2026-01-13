// Test const and static item parsing and codegen

const ANSWER: i32 = 42;
const COMPUTED: i32 = 10 + 5;
static COUNTER: i32 = 0;
static mut MUTABLE_COUNTER: i32 = 100;
pub const PUBLIC_CONST: i64 = 99;

fn main() -> i32 {
    // For now, just verify the file compiles
    // Accessing const/static values requires identifier lookup in globals
    // which we'll implement next
    19
}

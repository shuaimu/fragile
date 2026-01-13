// Test raw pointers

fn main() -> i32 {
    let x: i32 = 23;

    // Get address of x
    let ptr: *const i32 = &x as *const i32;

    // Dereference the pointer in unsafe block
    let val = unsafe { *ptr };

    val
}

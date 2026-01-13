// Test unsafe blocks

fn main() -> i32 {
    let x: i32 = 11;
    let y: i32 = 11;

    // Unsafe block - just treated as a regular block
    let sum = unsafe {
        x + y
    };

    sum
}

// Test mutable references - simpler version

fn main() -> i32 {
    let mut count: i32 = 33;
    let ptr = &mut count;
    *ptr = *ptr + 1;
    count  // Should return 34
}

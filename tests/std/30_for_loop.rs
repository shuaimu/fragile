// Test for loop (requires iterator support)

fn main() -> i32 {
    let mut sum = 0;
    // For now, test with while loop instead
    let mut i = 0;
    while i < 5 {
        sum = sum + i;
        i = i + 1;
    }
    sum  // 0 + 1 + 2 + 3 + 4 = 10
}

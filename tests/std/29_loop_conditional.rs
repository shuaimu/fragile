// Test loop with conditional break

fn main() -> i32 {
    let mut count = 0;
    let result = loop {
        count = count + 1;
        if count == 5 {
            break count * 10;
        }
    };
    result
}

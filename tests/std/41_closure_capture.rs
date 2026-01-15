// Test closure capturing a variable from outer scope
fn main() -> i32 {
    let x = 10;
    let add_x = |y: i32| x + y;  // Captures x
    add_x(31)  // Should return 41
}

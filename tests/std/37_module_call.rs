// Test calling functions from external modules
mod math;

fn main() -> i32 {
    // Call function from external module
    // math::add(20, 17) should return 37
    math::add(20, 17)
}

// Test generic enum (Option<T>) with monomorphization
enum Option<T> {
    None,
    Some(T),
}

fn main() -> i32 {
    let x: Option<i32> = Option::Some(38);
    match x {
        Option::None => 0,
        Option::Some(val) => val,  // Should extract and return 38
    }
}

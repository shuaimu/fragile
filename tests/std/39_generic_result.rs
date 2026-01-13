// Test generic enum with multiple type parameters (Result<T, E>)
enum Result<T, E> {
    Ok(T),
    Err(E),
}

fn main() -> i32 {
    let r: Result<i32, i32> = Result::Ok(39);
    match r {
        Result::Ok(val) => val,   // Should return 39
        Result::Err(e) => e + 100,
    }
}

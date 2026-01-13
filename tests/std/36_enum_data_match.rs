// Test matching enum variants with data extraction

enum Option {
    None,
    Some(i32),
}

fn main() -> i32 {
    let x = Option::Some(36);

    match x {
        Option::None => 0,
        Option::Some(val) => val,  // Should extract and return 36
    }
}

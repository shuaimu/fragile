unsafe extern "C" {
    fn fragile_demo_add(a: i32, b: i32) -> i32;
}

pub fn add(a: i32, b: i32) -> i32 {
    // The linked symbol is produced by the Fragile-built C++ static library.
    unsafe { fragile_demo_add(a, b) }
}

#[cfg(test)]
mod tests {
    use super::add;

    #[test]
    fn adds_two_numbers() {
        assert_eq!(add(20, 22), 42);
    }
}

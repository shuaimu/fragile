#[allow(warnings)]
mod fragile_demo_cpp {
    include!(concat!(env!("OUT_DIR"), "/fragile_demo_cpp.rs"));
}

pub fn add(a: i32, b: i32) -> i32 {
    fragile_demo_cpp::fragile_demo_add(a, b)
}

#[cfg(test)]
mod tests {
    use super::add;

    #[test]
    fn adds_two_numbers() {
        assert_eq!(add(20, 22), 42);
    }
}

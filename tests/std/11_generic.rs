// Generic function definition (parsed but not compiled without monomorphization)
fn identity<T>(x: T) -> T {
    x
}

// Bounded type parameters
fn clone_it<T: Clone>(x: T) -> T {
    x
}

// Multiple type parameters
fn swap<A, B>(a: A, b: B) -> (B, A) {
    (b, a)
}

fn main() -> i32 {
    // Can't call generic functions yet (needs monomorphization)
    42
}

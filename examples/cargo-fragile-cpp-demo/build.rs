fn main() {
    fragile_cargo::build_target("cpp/fragile.toml", "fragile_demo_cpp")
        .expect("failed to build C++ static library with Fragile");
}

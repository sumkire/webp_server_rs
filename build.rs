fn main() {
    println!("cargo:rustc-link-search=./deps/lib");
    println!("cargo:rustc-link-search=./lib");
}

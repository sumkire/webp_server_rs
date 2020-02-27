fn main() {
    println!("cargo:rustc-link-search=/usr/local/lib");
    println!("cargo:rustc-link-search=/usr/lib/x86_64-linux-gnu");
}

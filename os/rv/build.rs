// absolute path so the linker finds the script no matter where cargo runs from
fn main() {
    let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    println!("cargo:rustc-link-arg=-T{dir}/src/linker.ld");
    println!("cargo:rerun-if-changed=src/linker.ld");
}

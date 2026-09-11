fn main() {
    println!("cargo:rustc-link-search=native=/opt/homebrew/lib");
    println!("cargo:rustc-link-lib=git2");
    println!("cargo:rustc-link-lib=duckdb");
}

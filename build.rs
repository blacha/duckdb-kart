fn main() {
    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-link-arg=-undefined");
        println!("cargo:rustc-link-arg=dynamic_lookup");
    }

    #[cfg(target_os = "windows")]
    {
        println!("cargo:rustc-link-lib=duckdb");
        if let Ok(dir) = std::env::var("DUCKDB_LIB_DIR") {
            println!("cargo:rustc-link-search=native={}", dir);
        }
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
        println!("cargo:rustc-link-search=native={}/duckdb_win", manifest_dir);
        println!("cargo:rustc-link-search=native={}", manifest_dir);
    }
}

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("linux") {
        // The opt-in Linux helper requires the system OpenBLAS development
        // package at build time and its shared library at run time.
        println!("cargo:rustc-link-lib=openblas");
    }
}

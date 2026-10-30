fn main() {
    println!(
        "cargo:rustc-env=MAITTY_TARGET_TRIPLE={}",
        std::env::var("TARGET").expect("Cargo provides TARGET")
    );
    tauri_build::build()
}

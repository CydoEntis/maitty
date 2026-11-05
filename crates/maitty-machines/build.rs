#[path = "../../build.rs"]
#[allow(dead_code)]
mod app_brand;

fn main() {
    app_brand::configure(
        &std::path::PathBuf::from(
            std::env::var_os("CARGO_MANIFEST_DIR").expect("Cargo sets CARGO_MANIFEST_DIR"),
        )
        .join("../../apps/tui/Cargo.toml"),
    );
}

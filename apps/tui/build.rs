#[path = "../../build.rs"]
#[allow(dead_code)]
mod app_brand;

fn main() {
    app_brand::configure(std::path::Path::new("Cargo.toml"));
}

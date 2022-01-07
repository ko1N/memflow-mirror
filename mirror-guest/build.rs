extern crate winres;

fn main() {
    // compile with default values from Cargo.toml
    let mut res = winres::WindowsResource::new();
    res.set_icon("resources/icon.ico");
    res.compile().ok(); // this is allowed to fail in cross-compilation scenarios
}

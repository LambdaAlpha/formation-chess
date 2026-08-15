fn main() {
    // rust-embed embeds `static/` via `include_bytes!`, whose paths cargo does
    // not track as dependencies. Declare the folder here so any change to the
    // embedded frontend assets triggers a rebuild.
    println!("cargo:rerun-if-changed=static");
}

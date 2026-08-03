// The production app graph is deliberately absent from this target. The
// path-included module keeps the domain and runtime's pure transport tests
// independently compilable while `cargo check` validates production wiring.
#[path = "../src/codex_desktop/mod.rs"]
mod codex_desktop;

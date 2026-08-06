// The production app graph is deliberately absent from this target. The
// path-included module keeps the domain and runtime's pure transport tests
// independently compilable while normal library and Windows checks validate
// the complete production graph. This test-only copy intentionally exercises
// only a subset of that graph.
#[allow(dead_code)]
#[path = "../src/codex_desktop/mod.rs"]
mod codex_desktop;

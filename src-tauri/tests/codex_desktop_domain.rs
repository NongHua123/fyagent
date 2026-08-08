// The production app graph is deliberately absent from this target. The
// path-included module keeps the domain and runtime's pure transport tests
// independently compilable while normal library and Windows checks validate
// the complete production graph. This test-only copy intentionally exercises
// only a subset of that graph.
#[cfg(target_os = "windows")]
mod platform {
    pub(crate) mod process_launch {
        /// The path-included Windows deployment module names the production
        /// crate-root launcher. This isolated domain-test crate deliberately
        /// omits that graph, so any accidental launch attempt must fail closed;
        /// library and native Windows jobs validate the real implementation.
        pub(crate) fn launch_trusted_windows_app_aumid_as_user(_aumid: &str) -> Result<(), String> {
            Err("isolated Codex desktop domain tests cannot launch Windows apps".to_owned())
        }
    }
}

#[allow(dead_code)]
#[path = "../src/codex_desktop/mod.rs"]
mod codex_desktop;

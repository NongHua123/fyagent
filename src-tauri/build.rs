const WINDOWS_TEST_MANIFEST: &str = include_str!("windows/fyagent-test.manifest");
const WINDOWS_RELEASE_MANIFEST: &str = include_str!("windows/fyagent-release.manifest");

#[derive(Clone, Copy)]
enum WindowsManifest {
    Test,
    Release,
}

impl WindowsManifest {
    fn contents(self) -> &'static str {
        match self {
            Self::Test => WINDOWS_TEST_MANIFEST,
            Self::Release => WINDOWS_RELEASE_MANIFEST,
        }
    }
}

fn main() {
    println!("cargo:rustc-check-cfg=cfg(fyagent_windows_release)");

    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    if target_os != "windows" {
        tauri_build::build();
        return;
    }

    println!("cargo:rerun-if-env-changed=FYAGENT_WINDOWS_MANIFEST");
    println!("cargo:rerun-if-changed=windows/fyagent-test.manifest");
    println!("cargo:rerun-if-changed=windows/fyagent-release.manifest");

    let manifest = select_windows_manifest();
    if matches!(manifest, WindowsManifest::Release) {
        // The runtime uses this cfg for formal builds only; test/dev builds
        // must remain able to run without UAC.
        println!("cargo:rustc-cfg=fyagent_windows_release");
    }
    let windows = tauri_build::WindowsAttributes::new().app_manifest(manifest.contents());
    let attributes = tauri_build::Attributes::new().windows_attributes(windows);

    tauri_build::try_build(attributes).expect("failed to embed the FyAgent Windows manifest");
    embed_test_manifest();
}

/// `tauri-build` embeds the selected resource into the application binary.
/// Scope the ordinary-privilege manifest arguments to test targets so a formal
/// application binary receives only the release resource produced above.
fn embed_test_manifest() {
    let manifest_path = std::path::PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("missing CARGO_MANIFEST_DIR"),
    )
    .join("windows/fyagent-test.manifest");

    println!("cargo:rustc-link-arg-tests=/MANIFEST:EMBED");
    println!(
        "cargo:rustc-link-arg-tests=/MANIFESTINPUT:{}",
        manifest_path.display()
    );
}

fn select_windows_manifest() -> WindowsManifest {
    match std::env::var("FYAGENT_WINDOWS_MANIFEST") {
        Ok(value) if value == "release" => WindowsManifest::Release,
        Ok(value) if value == "test" || value == "dev" => WindowsManifest::Test,
        Ok(value) => {
            panic!("invalid FYAGENT_WINDOWS_MANIFEST={value:?}; expected release, test, or dev")
        }
        Err(std::env::VarError::NotPresent)
            if std::env::var("PROFILE").as_deref() == Ok("release") =>
        {
            // `cargo test --release` uses the same profile as a distributable
            // application, but must retain the ordinary-user test manifest.
            // A release application therefore has to opt in explicitly in the
            // signed release workflow; silently selecting either manifest here
            // would make one of those two binaries misleading.
            panic!(
                "FYAGENT_WINDOWS_MANIFEST must be explicitly set to release for a formal Windows release build or test for a release-profile test harness"
            )
        }
        Err(std::env::VarError::NotPresent) => WindowsManifest::Test,
        Err(error) => panic!("failed to read FYAGENT_WINDOWS_MANIFEST: {error}"),
    }
}

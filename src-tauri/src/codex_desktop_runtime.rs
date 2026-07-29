//! Application-owned construction of the Codex desktop installer service.
//!
//! This is intentionally outside `codex_desktop`: the domain module and its
//! transport adapters can be compiled in isolation, while this file owns the
//! dependency on the app service graph, log location, and current host.

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
use std::path::Path;
use std::sync::Arc;

#[cfg(any(target_os = "windows", target_os = "macos"))]
use crate::codex_desktop::platform::UnavailablePlatformAdapter;
use crate::codex_desktop::{
    download::HttpTransport,
    platform::CodexDesktopPlatform,
    runtime::{InstallerMetadataFetcher, InstallerTransportPurpose, RuntimeInstallerTransport},
    source::{AgentsMirrorSource, ReleaseSource},
    temp::JobTempDir,
    types::{CpuArchitecture, DesktopPlatform},
    verify::DiskSpaceProbe,
};
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
use crate::codex_desktop::{
    platform::UnsupportedPlatformAdapter,
    verify::{DiskSpaceProbeError, VolumeKey},
};
use crate::services::codex_desktop::{CodexDesktopService, CodexDesktopServiceDependencies};

/// Builds the one inert, process-local installer service used by `AppState`.
///
/// Constructing it starts no request, filesystem operation, local package
/// inspection, or installation. The two dedicated transports rebuild a clean
/// client only when the service actually resolves metadata or downloads a
/// release, so a runtime proxy change is observed per request.
pub(crate) fn production_service() -> CodexDesktopService {
    let user_agent = format!(
        "FyAgent/{} codex-desktop-installer",
        env!("CARGO_PKG_VERSION")
    );
    let metadata_transport: Arc<dyn HttpTransport> = Arc::new(RuntimeInstallerTransport::new(
        InstallerTransportPurpose::Metadata,
        user_agent.clone(),
    ));
    let source: Arc<dyn ReleaseSource> = Arc::new(AgentsMirrorSource::new(Arc::new(
        InstallerMetadataFetcher::new(metadata_transport),
    )));
    let download_transport: Arc<dyn HttpTransport> = Arc::new(RuntimeInstallerTransport::new(
        InstallerTransportPurpose::Download,
        user_agent,
    ));
    let (platform, disk_space_probe) = production_platform_dependencies();

    CodexDesktopService::new(CodexDesktopServiceDependencies::new(
        source,
        platform,
        download_transport,
        disk_space_probe,
        JobTempDir::system_root(),
        crate::panic_hook::get_log_dir(),
    ))
}

/// A deliberately unsuccessful probe for unsupported hosts or a platform
/// adapter that failed its construction trust gate. The service reaches it
/// only after the platform has already rejected the operation, but keeping a
/// real failing implementation avoids silently reporting sufficient storage.
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
#[derive(Debug, Default)]
struct UnavailableDiskSpaceProbe;

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
impl DiskSpaceProbe for UnavailableDiskSpaceProbe {
    fn volume_key(&self, _path: &Path) -> Result<VolumeKey, DiskSpaceProbeError> {
        Err(DiskSpaceProbeError::Unavailable)
    }

    fn available_bytes(&self, _volume: &VolumeKey) -> Result<u64, DiskSpaceProbeError> {
        Err(DiskSpaceProbeError::Unavailable)
    }
}

#[cfg(target_os = "windows")]
fn production_platform_dependencies() -> (Arc<dyn CodexDesktopPlatform>, Arc<dyn DiskSpaceProbe>) {
    use crate::codex_desktop::platform::windows::{
        current_official_publisher_evidence, SystemWindowsDiskSpaceProbe, WindowsPlatformAdapter,
    };

    let architecture = current_windows_architecture();
    let platform: Arc<dyn CodexDesktopPlatform> = match current_official_publisher_evidence()
        .and_then(WindowsPlatformAdapter::for_current_host)
    {
        Ok(adapter) => Arc::new(adapter),
        Err(error) => {
            log::warn!(
                "Codex desktop Windows adapter is unavailable until its trust gate is satisfied: {:?}",
                error.code()
            );
            Arc::new(UnavailablePlatformAdapter::new(
                DesktopPlatform::Windows,
                architecture,
                error,
            ))
        }
    };

    (platform, Arc::new(SystemWindowsDiskSpaceProbe::new()))
}

#[cfg(target_os = "windows")]
const fn current_windows_architecture() -> CpuArchitecture {
    #[cfg(target_arch = "x86_64")]
    {
        CpuArchitecture::X86_64
    }
    #[cfg(target_arch = "aarch64")]
    {
        CpuArchitecture::Aarch64
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        CpuArchitecture::Unsupported
    }
}

#[cfg(target_os = "macos")]
fn production_platform_dependencies() -> (Arc<dyn CodexDesktopPlatform>, Arc<dyn DiskSpaceProbe>) {
    use crate::codex_desktop::platform::macos::{MacosDiskSpaceProbe, MacosPlatformAdapter};

    let architecture = current_macos_architecture();
    let platform: Arc<dyn CodexDesktopPlatform> = match MacosPlatformAdapter::for_current_host() {
        Ok(adapter) => Arc::new(adapter),
        Err(error) => {
            log::warn!(
                "Codex desktop macOS adapter is unavailable because host discovery failed: {:?}",
                error.code()
            );
            Arc::new(UnavailablePlatformAdapter::new(
                DesktopPlatform::Macos,
                architecture,
                error,
            ))
        }
    };

    (platform, Arc::new(MacosDiskSpaceProbe::for_current_host()))
}

#[cfg(target_os = "macos")]
const fn current_macos_architecture() -> CpuArchitecture {
    #[cfg(target_arch = "aarch64")]
    {
        CpuArchitecture::Aarch64
    }
    #[cfg(target_arch = "x86_64")]
    {
        CpuArchitecture::X86_64UnsupportedMac
    }
    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
    {
        CpuArchitecture::Unsupported
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn production_platform_dependencies() -> (Arc<dyn CodexDesktopPlatform>, Arc<dyn DiskSpaceProbe>) {
    (
        Arc::new(UnsupportedPlatformAdapter::platform_unsupported(
            CpuArchitecture::Unsupported,
        )),
        Arc::new(UnavailableDiskSpaceProbe),
    )
}

//! AgentsMirror release metadata validation.
//!
//! The raw mirror schema intentionally stays private to this module.  In
//! particular, URL and delta fields are not represented here: a validated
//! release can only be downloaded through one of the fixed endpoint kinds.

use std::{
    collections::{BTreeMap, HashMap},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use futures::{future::BoxFuture, StreamExt};
use serde::Deserialize;

use super::{
    cancellation::{cancellation_error, race_with_cancellation, Cancellation},
    download::{BodyStream, TransportError},
    error::{InstallerError, InstallerErrorCode},
    types::{
        self, CpuArchitecture, DesktopPlatform, PlatformVersion, ReleaseDescriptor,
        TrustedDownloadEndpoint,
    },
    verify::{self, ArtifactKind},
};

const MAX_CHECKSUMS_BYTES: usize = 1024 * 1024;
const MAX_MANIFEST_BYTES: usize = 1024 * 1024;
const MAX_METADATA_ATTEMPTS: u8 = 3;
const MANIFEST_CHECKSUM_FILE_NAME: &str = "release-manifest.json";
pub const RELEASE_CACHE_TTL: Duration = Duration::from_secs(5 * 60);

/// Caller intent for the process-local, already-validated descriptor cache.
/// `ForceRefresh` is required on the install revalidation path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheMode {
    UseCache,
    ForceRefresh,
}

/// Metadata routes that the source may ask its HTTP adapter to retrieve.
/// Artifact endpoints are deliberately unavailable through this trait.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MetadataEndpoint {
    Checksums,
    Manifest,
}

impl MetadataEndpoint {
    const fn trusted_endpoint(self) -> TrustedDownloadEndpoint {
        match self {
            Self::Checksums => TrustedDownloadEndpoint::Checksums,
            Self::Manifest => TrustedDownloadEndpoint::Manifest,
        }
    }

    /// Fixed URL for an installer metadata document. No caller can provide a
    /// URL, and artifact routes are intentionally absent from this enum.
    pub(crate) const fn url(self) -> &'static str {
        self.trusted_endpoint().url()
    }

    pub(crate) const fn kind(self) -> &'static str {
        self.trusted_endpoint().kind()
    }
}

/// One metadata HTTP response. The source, rather than an adapter, owns the
/// bounded collection policy so an adapter cannot accidentally buffer an
/// unbounded response before the V5 parser sees it.
pub struct MetadataResponse {
    pub content_length: Option<u64>,
    pub body: BodyStream,
}

/// Narrow, object-safe boundary for the two release metadata documents.
/// The body is deliberately streamed so `AgentsMirrorSource` can apply its
/// one-mebibyte cap before allocating more metadata memory.
pub trait MetadataFetcher: Send + Sync {
    fn fetch<'a>(
        &'a self,
        endpoint: MetadataEndpoint,
    ) -> BoxFuture<'a, Result<MetadataResponse, TransportError>>;
}

/// Object-safe source boundary consumed by the installer service.
pub trait ReleaseSource: Send + Sync {
    fn resolve_latest<'a>(
        &'a self,
        platform: DesktopPlatform,
        architecture: CpuArchitecture,
        cache_mode: CacheMode,
        cancellation: &'a dyn Cancellation,
    ) -> BoxFuture<'a, Result<ReleaseDescriptor, InstallerError>>;
}

/// Clock injection keeps the five-minute cache deterministic in service tests.
pub trait ReleaseClock: Send + Sync {
    fn now(&self) -> Instant;
}

#[derive(Debug, Default)]
struct SystemReleaseClock;

impl ReleaseClock for SystemReleaseClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

/// A small injectable wait boundary keeps retry tests deterministic without
/// weakening the production backoff or cancellation behavior.
pub(crate) trait MetadataRetrySleeper: Send + Sync {
    fn sleep<'a>(&'a self, duration: Duration) -> BoxFuture<'a, ()>;
}

#[derive(Debug, Default)]
struct TokioMetadataRetrySleeper;

impl MetadataRetrySleeper for TokioMetadataRetrySleeper {
    fn sleep<'a>(&'a self, duration: Duration) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            tokio::time::sleep(duration).await;
        })
    }
}

/// The single V1 source: checksums first, then the raw manifest, followed by
/// fully local cross-validation.  It never retains remote artifact URLs.
pub struct AgentsMirrorSource {
    fetcher: Arc<dyn MetadataFetcher>,
    clock: Arc<dyn ReleaseClock>,
    retry_sleeper: Arc<dyn MetadataRetrySleeper>,
    cache: Mutex<ReleaseCache<ReleaseDescriptor>>,
}

impl AgentsMirrorSource {
    pub fn new(fetcher: Arc<dyn MetadataFetcher>) -> Self {
        Self::with_clock(fetcher, Arc::new(SystemReleaseClock))
    }

    pub fn with_clock(fetcher: Arc<dyn MetadataFetcher>, clock: Arc<dyn ReleaseClock>) -> Self {
        Self::with_dependencies(fetcher, clock, Arc::new(TokioMetadataRetrySleeper))
    }

    pub(crate) fn with_dependencies(
        fetcher: Arc<dyn MetadataFetcher>,
        clock: Arc<dyn ReleaseClock>,
        retry_sleeper: Arc<dyn MetadataRetrySleeper>,
    ) -> Self {
        Self {
            fetcher,
            clock,
            retry_sleeper,
            cache: Mutex::new(ReleaseCache::default()),
        }
    }

    async fn resolve_latest_inner(
        &self,
        platform: DesktopPlatform,
        architecture: CpuArchitecture,
        cache_mode: CacheMode,
        cancellation: &dyn Cancellation,
    ) -> Result<ReleaseDescriptor, InstallerError> {
        if cancellation.is_cancelled() {
            return Err(cancellation_error());
        }

        let target = raw_target(platform, architecture)?;
        if cache_mode == CacheMode::UseCache {
            let cached = self
                .cache
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .get(target, self.clock.now());
            if let Some(cached) = cached {
                return Ok(cached);
            }
        }

        let checksums = self
            .fetch_metadata(MetadataEndpoint::Checksums, cancellation)
            .await?;
        let manifest = self
            .fetch_metadata(MetadataEndpoint::Manifest, cancellation)
            .await?;
        if cancellation.is_cancelled() {
            return Err(cancellation_error());
        }
        let validated = validate_release_metadata(&checksums, &manifest, target)
            .map_err(map_validation_failure)?;
        let descriptor = release_descriptor_from_validated(validated)?;

        if cancellation.is_cancelled() {
            return Err(cancellation_error());
        }

        self.cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(target, descriptor.clone(), self.clock.now());
        Ok(descriptor)
    }

    async fn fetch_metadata(
        &self,
        endpoint: MetadataEndpoint,
        cancellation: &dyn Cancellation,
    ) -> Result<Vec<u8>, InstallerError> {
        for attempt in 1..=MAX_METADATA_ATTEMPTS {
            let response = match race_with_cancellation(self.fetcher.fetch(endpoint), cancellation)
                .await
            {
                Ok(Ok(response)) => response,
                Ok(Err(error)) if error.is_retryable() && attempt < MAX_METADATA_ATTEMPTS => {
                    self.wait_for_metadata_retry(endpoint, attempt, cancellation)
                        .await?;
                    continue;
                }
                Ok(Err(error)) => return Err(metadata_transport_error(error, endpoint, attempt)),
                Err(_) => return Err(metadata_cancellation_error(endpoint, attempt)),
            };

            match race_with_cancellation(
                collect_metadata_response(response, metadata_response_limit(endpoint)),
                cancellation,
            )
            .await
            {
                Ok(Ok(bytes)) => return Ok(bytes),
                Ok(Err(MetadataResponseReadError::Transport(error)))
                    if error.is_retryable() && attempt < MAX_METADATA_ATTEMPTS =>
                {
                    self.wait_for_metadata_retry(endpoint, attempt, cancellation)
                        .await?;
                }
                Ok(Err(MetadataResponseReadError::Transport(error))) => {
                    return Err(metadata_transport_error(error, endpoint, attempt));
                }
                Ok(Err(MetadataResponseReadError::TooLarge)) => {
                    return Err(metadata_too_large_error(endpoint, attempt));
                }
                Err(_) => return Err(metadata_cancellation_error(endpoint, attempt)),
            }
        }

        Err(InstallerError::new(InstallerErrorCode::InternalError)
            .with_diagnostic_message("metadata retry loop exited unexpectedly")
            .with_endpoint_kind(endpoint.kind()))
    }

    async fn wait_for_metadata_retry(
        &self,
        endpoint: MetadataEndpoint,
        failed_attempt: u8,
        cancellation: &dyn Cancellation,
    ) -> Result<(), InstallerError> {
        let delay = metadata_retry_delay_after(failed_attempt);
        race_with_cancellation(self.retry_sleeper.sleep(delay), cancellation)
            .await
            .map_err(|_| metadata_cancellation_error(endpoint, failed_attempt))
    }
}

impl ReleaseSource for AgentsMirrorSource {
    fn resolve_latest<'a>(
        &'a self,
        platform: DesktopPlatform,
        architecture: CpuArchitecture,
        cache_mode: CacheMode,
        cancellation: &'a dyn Cancellation,
    ) -> BoxFuture<'a, Result<ReleaseDescriptor, InstallerError>> {
        Box::pin(async move {
            self.resolve_latest_inner(platform, architecture, cache_mode, cancellation)
                .await
        })
    }
}

#[derive(Debug)]
enum MetadataResponseReadError {
    TooLarge,
    Transport(TransportError),
}

async fn collect_metadata_response(
    mut response: MetadataResponse,
    maximum_bytes: usize,
) -> Result<Vec<u8>, MetadataResponseReadError> {
    if response
        .content_length
        .is_some_and(|content_length| content_length > maximum_bytes as u64)
    {
        return Err(MetadataResponseReadError::TooLarge);
    }

    let mut bytes = Vec::new();
    while let Some(chunk) = response.body.next().await {
        let chunk = chunk.map_err(MetadataResponseReadError::Transport)?;
        let collected_length = bytes
            .len()
            .checked_add(chunk.len())
            .ok_or(MetadataResponseReadError::TooLarge)?;
        if collected_length > maximum_bytes {
            return Err(MetadataResponseReadError::TooLarge);
        }
        bytes.extend_from_slice(&chunk);
    }

    Ok(bytes)
}

const fn metadata_response_limit(endpoint: MetadataEndpoint) -> usize {
    match endpoint {
        MetadataEndpoint::Checksums => MAX_CHECKSUMS_BYTES,
        MetadataEndpoint::Manifest => MAX_MANIFEST_BYTES,
    }
}

fn metadata_retry_delay_after(failed_attempt: u8) -> Duration {
    match failed_attempt {
        1 => Duration::from_secs(1),
        _ => Duration::from_secs(3),
    }
}

fn metadata_transport_error(
    error: TransportError,
    endpoint: MetadataEndpoint,
    attempt: u8,
) -> InstallerError {
    let code = if error.is_redirect_rejected() {
        InstallerErrorCode::RedirectRejected
    } else {
        InstallerErrorCode::SourceUnavailable
    };

    InstallerError::new(code)
        .with_diagnostic_message(error.diagnostic())
        .with_endpoint_kind(endpoint.kind())
        .with_attempt(attempt, MAX_METADATA_ATTEMPTS)
}

fn metadata_too_large_error(endpoint: MetadataEndpoint, attempt: u8) -> InstallerError {
    InstallerError::new(InstallerErrorCode::ReleaseMetadataInvalid)
        .with_diagnostic_message("metadata response exceeded the one-mebibyte limit")
        .with_endpoint_kind(endpoint.kind())
        .with_attempt(attempt, MAX_METADATA_ATTEMPTS)
}

fn metadata_cancellation_error(endpoint: MetadataEndpoint, attempt: u8) -> InstallerError {
    cancellation_error()
        .with_endpoint_kind(endpoint.kind())
        .with_attempt(attempt, MAX_METADATA_ATTEMPTS)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum RawTarget {
    WindowsX64,
    WindowsArm64,
    MacosArm64,
}

#[derive(Debug, Clone)]
struct CachedRelease<T> {
    release: T,
    resolved_at: Instant,
}

#[derive(Debug)]
struct ReleaseCache<T> {
    entries: HashMap<RawTarget, CachedRelease<T>>,
}

impl<T> Default for ReleaseCache<T> {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }
}

impl<T: Clone> ReleaseCache<T> {
    fn get(&self, target: RawTarget, now: Instant) -> Option<T> {
        let cached = self.entries.get(&target)?;
        let age = now.checked_duration_since(cached.resolved_at)?;
        (age < RELEASE_CACHE_TTL).then(|| cached.release.clone())
    }

    fn insert(&mut self, target: RawTarget, release: T, now: Instant) {
        self.entries.insert(
            target,
            CachedRelease {
                release,
                resolved_at: now,
            },
        );
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ValidatedPlatformVersion {
    WindowsMsix(String),
    MacBundle(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ValidatedRelease {
    target: RawTarget,
    display_version: String,
    platform_version: ValidatedPlatformVersion,
    artifact_file_name: String,
    expected_sha256: String,
    expected_size: u64,
    minimum_os_version: Option<String>,
}

/// Deliberately contains only controlled descriptions.  Remote field values,
/// including arbitrary URLs, must not become diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SourceValidationFailure {
    MetadataInvalid(&'static str),
    ReleaseNotAvailable,
    ChecksumMissing(&'static str),
    ManifestChecksumMismatch,
    ArtifactChecksumMismatch,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawReleaseManifest {
    schema_version: u64,
    generated_at: String,
    codex_version: String,
    published_at: String,
    sources: RawSources,
    derived: RawDerived,
}

#[derive(Debug, Deserialize)]
struct RawSources {
    windows: Option<RawWindowsSource>,
    macos: Option<RawMacosSource>,
}

#[derive(Debug, Deserialize)]
struct RawWindowsSource {
    architectures: RawWindowsArchitectures,
}

#[derive(Debug, Deserialize)]
struct RawWindowsArchitectures {
    x64: Option<RawWindowsArtifact>,
    arm64: Option<RawWindowsArtifact>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawWindowsArtifact {
    architecture: Option<String>,
    status: Option<String>,
    downloadable: Option<bool>,
    version: Option<String>,
    package_moniker: Option<String>,
    content_length: Option<u64>,
    sha256: Option<String>,
    minimum_os_version: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawMacosSource {
    arm64: Option<RawMacosArtifact>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawMacosArtifact {
    content_length: Option<u64>,
    bundle_short_version: Option<String>,
    bundle_version: Option<String>,
    bundle_identifier: Option<String>,
    team_identifier: Option<String>,
    sha256: Option<String>,
    downloadable: Option<bool>,
    status: Option<String>,
    minimum_os_version: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawDerived {
    latest_checksums: BTreeMap<String, String>,
}

fn validate_release_metadata(
    checksum_bytes: &[u8],
    manifest_bytes: &[u8],
    target: RawTarget,
) -> Result<ValidatedRelease, SourceValidationFailure> {
    let checksums = parse_checksums(checksum_bytes)?;
    verify_manifest_checksum(&checksums, manifest_bytes)?;

    let manifest = parse_manifest(manifest_bytes)?;
    match target {
        RawTarget::WindowsX64 => validate_windows_release(&manifest, &checksums, "x64"),
        RawTarget::WindowsArm64 => validate_windows_release(&manifest, &checksums, "arm64"),
        RawTarget::MacosArm64 => validate_macos_release(&manifest, &checksums),
    }
}

fn parse_checksums(bytes: &[u8]) -> Result<BTreeMap<String, String>, SourceValidationFailure> {
    if bytes.len() > MAX_CHECKSUMS_BYTES {
        return Err(SourceValidationFailure::MetadataInvalid(
            "checksums response is too large",
        ));
    }

    let text = std::str::from_utf8(bytes).map_err(|_| {
        SourceValidationFailure::MetadataInvalid("checksums response is not valid UTF-8")
    })?;
    let mut entries = BTreeMap::new();

    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }

        let (checksum, filename) = parse_checksum_line(line)?;
        if entries
            .insert(filename.to_owned(), checksum.to_owned())
            .is_some()
        {
            return Err(SourceValidationFailure::MetadataInvalid(
                "checksums response contains a duplicate filename",
            ));
        }
    }

    Ok(entries)
}

fn parse_checksum_line(line: &str) -> Result<(&str, &str), SourceValidationFailure> {
    let bytes = line.as_bytes();
    if bytes.len() < 66 || !bytes[..64].iter().all(u8::is_ascii_hexdigit) {
        return Err(SourceValidationFailure::MetadataInvalid(
            "checksums response contains an invalid SHA-256 line",
        ));
    }

    let checksum = &line[..64];
    let filename = match &line[64..] {
        remainder if remainder.starts_with("  ") => &remainder[2..],
        remainder if remainder.starts_with(" *") => &remainder[2..],
        _ => {
            return Err(SourceValidationFailure::MetadataInvalid(
                "checksums response contains an invalid SHA-256 separator",
            ));
        }
    };

    validate_checksum_file_name(filename)?;
    Ok((checksum, filename))
}

fn verify_manifest_checksum(
    checksums: &BTreeMap<String, String>,
    manifest_bytes: &[u8],
) -> Result<(), SourceValidationFailure> {
    let expected = checksums
        .get(MANIFEST_CHECKSUM_FILE_NAME)
        .ok_or(SourceValidationFailure::ChecksumMissing("release manifest"))?;
    let expected = normalize_metadata_sha256(expected)?;
    let matches = verify::sha256_matches(manifest_bytes, &expected).map_err(|_| {
        SourceValidationFailure::MetadataInvalid(
            "release metadata contains an invalid SHA-256 value",
        )
    })?;

    if !matches {
        return Err(SourceValidationFailure::ManifestChecksumMismatch);
    }

    Ok(())
}

fn parse_manifest(bytes: &[u8]) -> Result<RawReleaseManifest, SourceValidationFailure> {
    if bytes.len() > MAX_MANIFEST_BYTES {
        return Err(SourceValidationFailure::MetadataInvalid(
            "release manifest response is too large",
        ));
    }

    let manifest = serde_json::from_slice::<RawReleaseManifest>(bytes).map_err(|_| {
        SourceValidationFailure::MetadataInvalid("release manifest is not valid schema-v5 JSON")
    })?;

    if manifest.schema_version != 5 {
        return Err(SourceValidationFailure::MetadataInvalid(
            "release manifest schema version is unsupported",
        ));
    }

    require_non_empty(&manifest.generated_at, "release manifest generatedAt")?;
    require_non_empty(&manifest.codex_version, "release manifest codexVersion")?;
    require_non_empty(&manifest.published_at, "release manifest publishedAt")?;
    Ok(manifest)
}

fn validate_windows_release(
    manifest: &RawReleaseManifest,
    checksums: &BTreeMap<String, String>,
    expected_architecture: &str,
) -> Result<ValidatedRelease, SourceValidationFailure> {
    let architectures = &manifest
        .sources
        .windows
        .as_ref()
        .ok_or(SourceValidationFailure::ReleaseNotAvailable)?
        .architectures;
    let artifact = match expected_architecture {
        "x64" => architectures.x64.as_ref(),
        "arm64" => architectures.arm64.as_ref(),
        _ => None,
    }
    .ok_or(SourceValidationFailure::ReleaseNotAvailable)?;

    validate_downloadable(artifact.downloadable, artifact.status.as_deref())?;
    if artifact.architecture.as_deref() != Some(expected_architecture) {
        return Err(SourceValidationFailure::MetadataInvalid(
            "Windows architecture branch does not match its declared architecture",
        ));
    }

    let version = require_non_empty_option(artifact.version.as_deref(), "Windows version")?;
    let platform_version = validate_windows_version(version)?;
    let package_moniker = require_non_empty_option(
        artifact.package_moniker.as_deref(),
        "Windows packageMoniker",
    )?;
    validate_package_moniker(package_moniker, expected_architecture, version)?;
    let artifact_file_name = format!("{package_moniker}.Msix");
    validate_release_artifact_file_name(&artifact_file_name, ArtifactKind::Msix)?;
    let expected_size = require_non_zero(artifact.content_length, "Windows contentLength")?;
    let expected_sha256 = resolve_expected_checksum(
        &manifest.derived.latest_checksums,
        checksums,
        &artifact_file_name,
        artifact.sha256.as_deref(),
    )?;

    Ok(ValidatedRelease {
        target: if expected_architecture == "x64" {
            RawTarget::WindowsX64
        } else {
            RawTarget::WindowsArm64
        },
        // The manifest-wide codexVersion may describe a different platform's
        // release. The Windows card must display the validated MSIX version
        // that this architecture can actually install.
        display_version: version.to_owned(),
        platform_version,
        artifact_file_name,
        expected_sha256,
        expected_size,
        minimum_os_version: optional_non_empty_string(
            artifact.minimum_os_version.as_deref(),
            "Windows minimumOsVersion",
        )?,
    })
}

fn validate_macos_release(
    manifest: &RawReleaseManifest,
    checksums: &BTreeMap<String, String>,
) -> Result<ValidatedRelease, SourceValidationFailure> {
    let artifact = manifest
        .sources
        .macos
        .as_ref()
        .and_then(|source| source.arm64.as_ref())
        .ok_or(SourceValidationFailure::ReleaseNotAvailable)?;

    validate_downloadable(artifact.downloadable, artifact.status.as_deref())?;
    let display_version = require_non_empty_option(
        artifact.bundle_short_version.as_deref(),
        "macOS bundleShortVersion",
    )?;
    let bundle_version =
        require_non_empty_option(artifact.bundle_version.as_deref(), "macOS bundleVersion")?;
    require_non_empty_option(
        artifact.bundle_identifier.as_deref(),
        "macOS bundleIdentifier",
    )?;
    require_non_empty_option(artifact.team_identifier.as_deref(), "macOS teamIdentifier")?;
    let expected_size = require_non_zero(artifact.content_length, "macOS contentLength")?;
    let artifact_file_name = derive_macos_arm64_artifact_file_name(
        &manifest.derived.latest_checksums,
        artifact.sha256.as_deref(),
    )?;
    let expected_sha256 = resolve_expected_checksum(
        &manifest.derived.latest_checksums,
        checksums,
        &artifact_file_name,
        artifact.sha256.as_deref(),
    )?;

    Ok(ValidatedRelease {
        target: RawTarget::MacosArm64,
        display_version: display_version.to_owned(),
        platform_version: validate_mac_bundle_version(bundle_version)?,
        artifact_file_name,
        expected_sha256,
        expected_size,
        minimum_os_version: optional_non_empty_string(
            artifact.minimum_os_version.as_deref(),
            "macOS minimumOsVersion",
        )?,
    })
}

fn validate_downloadable(
    downloadable: Option<bool>,
    status: Option<&str>,
) -> Result<(), SourceValidationFailure> {
    match (downloadable, status) {
        (Some(true), Some("downloadable")) => Ok(()),
        (Some(false), Some(_)) => Err(SourceValidationFailure::ReleaseNotAvailable),
        (Some(true), Some(_)) => Err(SourceValidationFailure::MetadataInvalid(
            "release status conflicts with downloadable flag",
        )),
        _ => Err(SourceValidationFailure::MetadataInvalid(
            "release is missing downloadable status fields",
        )),
    }
}

fn resolve_expected_checksum(
    derived_checksums: &BTreeMap<String, String>,
    checksums: &BTreeMap<String, String>,
    artifact_file_name: &str,
    branch_checksum: Option<&str>,
) -> Result<String, SourceValidationFailure> {
    let derived = derived_checksums.get(artifact_file_name).ok_or(
        SourceValidationFailure::ChecksumMissing("derived artifact checksum"),
    )?;
    let checksum_file =
        checksums
            .get(artifact_file_name)
            .ok_or(SourceValidationFailure::ChecksumMissing(
                "artifact checksum",
            ))?;
    let derived = normalize_metadata_sha256(derived)?;
    let checksum_file = normalize_metadata_sha256(checksum_file)?;

    if derived != checksum_file {
        return Err(SourceValidationFailure::ArtifactChecksumMismatch);
    }

    if let Some(branch_checksum) = branch_checksum {
        let branch_checksum = normalize_metadata_sha256(branch_checksum)?;
        if branch_checksum != derived {
            return Err(SourceValidationFailure::ArtifactChecksumMismatch);
        }
    }

    Ok(derived)
}

fn validate_windows_version(
    value: &str,
) -> Result<ValidatedPlatformVersion, SourceValidationFailure> {
    PlatformVersion::parse_windows_msix(value).map_err(|_| {
        SourceValidationFailure::MetadataInvalid("Windows version is not a valid MSIX version")
    })?;
    Ok(ValidatedPlatformVersion::WindowsMsix(value.to_owned()))
}

fn validate_mac_bundle_version(
    value: &str,
) -> Result<ValidatedPlatformVersion, SourceValidationFailure> {
    PlatformVersion::parse_mac_bundle(value.to_owned()).map_err(|_| {
        SourceValidationFailure::MetadataInvalid("macOS bundleVersion is not comparable")
    })?;
    Ok(ValidatedPlatformVersion::MacBundle(value.to_owned()))
}

fn validate_package_moniker(
    package_moniker: &str,
    expected_architecture: &str,
    version: &str,
) -> Result<(), SourceValidationFailure> {
    validate_package_moniker_component(package_moniker)?;
    if package_moniker.to_ascii_lowercase().ends_with(".msix")
        || !package_moniker.contains(&format!("_{expected_architecture}_"))
        || !package_moniker.contains(&format!("_{version}_"))
    {
        return Err(SourceValidationFailure::MetadataInvalid(
            "Windows packageMoniker is inconsistent with its branch",
        ));
    }

    Ok(())
}

fn validate_checksum_file_name(value: &str) -> Result<(), SourceValidationFailure> {
    if value.is_empty()
        || value.trim() != value
        || value.contains(['/', '\\', '\0'])
        || value.contains("..")
        || value.chars().any(char::is_control)
    {
        return Err(SourceValidationFailure::MetadataInvalid(
            "release metadata contains an unsafe artifact filename",
        ));
    }

    Ok(())
}

fn validate_release_artifact_file_name(
    value: &str,
    kind: ArtifactKind,
) -> Result<(), SourceValidationFailure> {
    verify::validate_artifact_file_name(value, kind).map_err(|_| {
        SourceValidationFailure::MetadataInvalid(
            "release metadata contains an unsafe artifact filename",
        )
    })
}

fn validate_package_moniker_component(value: &str) -> Result<(), SourceValidationFailure> {
    if value.is_empty()
        || value.trim() != value
        || value.contains(['/', '\\', '\0'])
        || value.contains("..")
        || value.chars().any(char::is_control)
    {
        return Err(SourceValidationFailure::MetadataInvalid(
            "Windows packageMoniker is unsafe",
        ));
    }

    Ok(())
}

fn require_non_empty(value: &str, name: &'static str) -> Result<(), SourceValidationFailure> {
    if value.is_empty() || value.trim() != value || value.chars().any(char::is_control) {
        return Err(SourceValidationFailure::MetadataInvalid(name));
    }
    Ok(())
}

fn require_non_empty_option<'a>(
    value: Option<&'a str>,
    name: &'static str,
) -> Result<&'a str, SourceValidationFailure> {
    let value = value.ok_or(SourceValidationFailure::MetadataInvalid(name))?;
    require_non_empty(value, name)?;
    Ok(value)
}

fn require_non_zero(
    value: Option<u64>,
    name: &'static str,
) -> Result<u64, SourceValidationFailure> {
    match value {
        Some(value) if value > 0 => Ok(value),
        _ => Err(SourceValidationFailure::MetadataInvalid(name)),
    }
}

fn optional_non_empty_string(
    value: Option<&str>,
    name: &'static str,
) -> Result<Option<String>, SourceValidationFailure> {
    match value {
        Some(value) => {
            require_non_empty(value, name)?;
            Ok(Some(value.to_owned()))
        }
        None => Ok(None),
    }
}

fn normalize_metadata_sha256(value: &str) -> Result<String, SourceValidationFailure> {
    if value.trim() != value {
        return Err(SourceValidationFailure::MetadataInvalid(
            "release metadata contains an invalid SHA-256 value",
        ));
    }
    types::normalize_sha256(value).map_err(|_| {
        SourceValidationFailure::MetadataInvalid(
            "release metadata contains an invalid SHA-256 value",
        )
    })
}

/// The fixed download endpoint intentionally does not imply a fixed macOS
/// filename. The V5 branch either pins its digest directly or, for older
/// metadata, must have exactly one safe DMG checksum entry. This keeps a
/// renamed official artifact usable without allowing a remote URL or arbitrary
/// filesystem path into the descriptor.
fn derive_macos_arm64_artifact_file_name(
    derived_checksums: &BTreeMap<String, String>,
    branch_checksum: Option<&str>,
) -> Result<String, SourceValidationFailure> {
    let expected_branch_checksum = branch_checksum.map(normalize_metadata_sha256).transpose()?;
    let mut safe_candidates = Vec::new();
    let mut branch_candidates = Vec::new();

    for (file_name, derived_checksum) in derived_checksums {
        if !file_name
            .rsplit_once('.')
            .is_some_and(|(_, extension)| extension.eq_ignore_ascii_case("dmg"))
        {
            continue;
        }

        validate_release_artifact_file_name(file_name, ArtifactKind::Dmg)?;
        safe_candidates.push(file_name);
        if let Some(expected_branch_checksum) = expected_branch_checksum.as_deref() {
            let derived_checksum = normalize_metadata_sha256(derived_checksum)?;
            if derived_checksum != expected_branch_checksum {
                continue;
            }
        }
        branch_candidates.push(file_name);
    }

    match branch_candidates.as_slice() {
        [file_name] => Ok((*file_name).to_owned()),
        [] if expected_branch_checksum.is_some() && !safe_candidates.is_empty() => {
            Err(SourceValidationFailure::ArtifactChecksumMismatch)
        }
        [] => Err(SourceValidationFailure::ChecksumMissing(
            "macOS ARM64 artifact checksum",
        )),
        _ => Err(SourceValidationFailure::MetadataInvalid(
            "macOS ARM64 artifact filename is ambiguous",
        )),
    }
}

fn raw_target(
    platform: DesktopPlatform,
    architecture: CpuArchitecture,
) -> Result<RawTarget, InstallerError> {
    match (platform, architecture) {
        (DesktopPlatform::Windows, CpuArchitecture::X86_64) => Ok(RawTarget::WindowsX64),
        (DesktopPlatform::Windows, CpuArchitecture::Aarch64) => Ok(RawTarget::WindowsArm64),
        (DesktopPlatform::Macos, CpuArchitecture::Aarch64) => Ok(RawTarget::MacosArm64),
        _ => Err(
            InstallerError::new(InstallerErrorCode::ArchitectureUnsupported)
                .with_context("architecture", architecture.as_str())
                .with_diagnostic_message(
                    "release source has no package for this platform architecture",
                ),
        ),
    }
}

fn release_descriptor_from_validated(
    validated: ValidatedRelease,
) -> Result<ReleaseDescriptor, InstallerError> {
    let (platform, architecture, endpoint) = match validated.target {
        RawTarget::WindowsX64 => (
            DesktopPlatform::Windows,
            CpuArchitecture::X86_64,
            TrustedDownloadEndpoint::WinX64,
        ),
        RawTarget::WindowsArm64 => (
            DesktopPlatform::Windows,
            CpuArchitecture::Aarch64,
            TrustedDownloadEndpoint::WinArm64,
        ),
        RawTarget::MacosArm64 => (
            DesktopPlatform::Macos,
            CpuArchitecture::Aarch64,
            TrustedDownloadEndpoint::MacArm64,
        ),
    };
    let platform_version = match validated.platform_version {
        ValidatedPlatformVersion::WindowsMsix(value) => {
            PlatformVersion::parse_windows_msix(&value)?
        }
        ValidatedPlatformVersion::MacBundle(value) => PlatformVersion::parse_mac_bundle(value)?,
    };

    ReleaseDescriptor::new(
        platform,
        architecture,
        validated.display_version,
        platform_version,
        validated.artifact_file_name,
        validated.expected_sha256,
        validated.expected_size,
        endpoint,
        validated.minimum_os_version,
    )
}

fn map_validation_failure(failure: SourceValidationFailure) -> InstallerError {
    let (code, endpoint_kind, message) = match failure {
        SourceValidationFailure::MetadataInvalid(message) => (
            InstallerErrorCode::ReleaseMetadataInvalid,
            if message.starts_with("checksums") {
                MetadataEndpoint::Checksums.kind()
            } else {
                MetadataEndpoint::Manifest.kind()
            },
            message,
        ),
        SourceValidationFailure::ReleaseNotAvailable => (
            InstallerErrorCode::ReleaseNotAvailable,
            MetadataEndpoint::Manifest.kind(),
            "release is not downloadable for this platform architecture",
        ),
        SourceValidationFailure::ChecksumMissing(message) => (
            InstallerErrorCode::ChecksumMissing,
            MetadataEndpoint::Checksums.kind(),
            message,
        ),
        SourceValidationFailure::ManifestChecksumMismatch => (
            InstallerErrorCode::ChecksumMismatch,
            MetadataEndpoint::Manifest.kind(),
            "release manifest did not match its checksum entry",
        ),
        SourceValidationFailure::ArtifactChecksumMismatch => (
            InstallerErrorCode::ChecksumMismatch,
            MetadataEndpoint::Manifest.kind(),
            "release artifact checksum sources disagreed",
        ),
    };

    InstallerError::new(code)
        .with_endpoint_kind(endpoint_kind)
        .with_diagnostic_message(message)
}

#[cfg(test)]
mod tests {
    use std::{
        collections::{HashMap, VecDeque},
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc, Mutex,
        },
    };

    use bytes::Bytes;
    use futures::{future, stream};

    use super::*;
    use crate::codex_desktop::error::SuggestedAction;

    const VALID_MANIFEST: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/codex_desktop/agentsmirror-v5-valid.json"
    ));
    const VALID_CHECKSUMS: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/codex_desktop/agentsmirror-v5-valid-checksums.txt"
    ));

    fn checksums_for(manifest: &[u8]) -> Vec<u8> {
        checksums_for_with_macos_artifacts(manifest, &[("Codex-mac-arm64.dmg", &"c".repeat(64))])
    }

    fn checksums_for_with_macos_artifacts(
        manifest: &[u8],
        macos_artifacts: &[(&str, &str)],
    ) -> Vec<u8> {
        let macos_lines = macos_artifacts
            .iter()
            .map(|(file_name, checksum)| format!("{checksum} *{file_name}\n"))
            .collect::<String>();
        format!(
            "{}  release-manifest.json\n{}  OpenAI.Codex_1.2.3.4_x64__fixture.Msix\n{} *OpenAI.Codex_1.2.3.4_arm64__fixture.Msix\n{macos_lines}",
            verify::sha256_hex(manifest),
            "a".repeat(64),
            "b".repeat(64),
        )
        .into_bytes()
    }

    fn manifest_value() -> serde_json::Value {
        serde_json::from_slice(VALID_MANIFEST).expect("fixture must contain JSON")
    }

    #[derive(Clone)]
    struct FakeMetadataResponse {
        content_length: Option<u64>,
        chunks: Vec<Result<Bytes, TransportError>>,
    }

    impl FakeMetadataResponse {
        fn from_bytes(bytes: impl Into<Vec<u8>>) -> Self {
            let bytes = bytes.into();
            Self {
                content_length: Some(bytes.len() as u64),
                chunks: vec![Ok(Bytes::from(bytes))],
            }
        }

        fn with_chunks(
            content_length: Option<u64>,
            chunks: Vec<Result<Bytes, TransportError>>,
        ) -> Self {
            Self {
                content_length,
                chunks,
            }
        }

        fn into_response(self) -> MetadataResponse {
            MetadataResponse {
                content_length: self.content_length,
                body: Box::pin(stream::iter(self.chunks)),
            }
        }
    }

    #[derive(Default)]
    struct FakeMetadataFetcher {
        responses: Mutex<HashMap<MetadataEndpoint, FakeMetadataResponse>>,
        queued_responses: Mutex<
            HashMap<MetadataEndpoint, VecDeque<Result<FakeMetadataResponse, TransportError>>>,
        >,
        calls: Mutex<Vec<MetadataEndpoint>>,
    }

    impl FakeMetadataFetcher {
        fn from_fixture() -> Self {
            Self {
                responses: Mutex::new(HashMap::from([
                    (
                        MetadataEndpoint::Checksums,
                        FakeMetadataResponse::from_bytes(VALID_CHECKSUMS),
                    ),
                    (
                        MetadataEndpoint::Manifest,
                        FakeMetadataResponse::from_bytes(VALID_MANIFEST),
                    ),
                ])),
                queued_responses: Mutex::new(HashMap::new()),
                calls: Mutex::new(Vec::new()),
            }
        }

        fn replace_manifest(&self, bytes: Vec<u8>) {
            self.responses.lock().unwrap().insert(
                MetadataEndpoint::Manifest,
                FakeMetadataResponse::from_bytes(bytes),
            );
        }

        fn replace_checksums(&self, response: FakeMetadataResponse) {
            self.responses
                .lock()
                .unwrap()
                .insert(MetadataEndpoint::Checksums, response);
        }

        fn queue_transport_error(&self, endpoint: MetadataEndpoint, error: TransportError) {
            self.queued_responses
                .lock()
                .unwrap()
                .entry(endpoint)
                .or_default()
                .push_back(Err(error));
        }

        fn queue_response(&self, endpoint: MetadataEndpoint, response: FakeMetadataResponse) {
            self.queued_responses
                .lock()
                .unwrap()
                .entry(endpoint)
                .or_default()
                .push_back(Ok(response));
        }

        fn calls(&self) -> Vec<MetadataEndpoint> {
            self.calls.lock().unwrap().clone()
        }
    }

    impl MetadataFetcher for FakeMetadataFetcher {
        fn fetch<'a>(
            &'a self,
            endpoint: MetadataEndpoint,
        ) -> BoxFuture<'a, Result<MetadataResponse, TransportError>> {
            self.calls.lock().unwrap().push(endpoint);
            let queued_response = self
                .queued_responses
                .lock()
                .unwrap()
                .get_mut(&endpoint)
                .and_then(VecDeque::pop_front);
            let response = queued_response.or_else(|| {
                self.responses
                    .lock()
                    .unwrap()
                    .get(&endpoint)
                    .cloned()
                    .map(Ok)
            });
            Box::pin(async move {
                response
                    .unwrap_or_else(|| {
                        Err(TransportError::non_retryable(
                            "fake metadata response is missing",
                        ))
                    })
                    .map(FakeMetadataResponse::into_response)
            })
        }
    }

    #[derive(Default)]
    struct PendingMetadataFetcher {
        calls: Mutex<Vec<MetadataEndpoint>>,
    }

    impl PendingMetadataFetcher {
        fn calls(&self) -> Vec<MetadataEndpoint> {
            self.calls.lock().unwrap().clone()
        }
    }

    impl MetadataFetcher for PendingMetadataFetcher {
        fn fetch<'a>(
            &'a self,
            endpoint: MetadataEndpoint,
        ) -> BoxFuture<'a, Result<MetadataResponse, TransportError>> {
            self.calls.lock().unwrap().push(endpoint);
            Box::pin(future::pending())
        }
    }

    struct FakeClock {
        now: Mutex<Instant>,
    }

    impl FakeClock {
        fn new(now: Instant) -> Self {
            Self {
                now: Mutex::new(now),
            }
        }

        fn advance(&self, duration: Duration) {
            let mut now = self.now.lock().unwrap();
            *now += duration;
        }
    }

    impl ReleaseClock for FakeClock {
        fn now(&self) -> Instant {
            *self.now.lock().unwrap()
        }
    }

    #[derive(Default)]
    struct FakeMetadataRetrySleeper {
        delays: Mutex<Vec<Duration>>,
    }

    impl FakeMetadataRetrySleeper {
        fn delays(&self) -> Vec<Duration> {
            self.delays.lock().unwrap().clone()
        }
    }

    impl MetadataRetrySleeper for FakeMetadataRetrySleeper {
        fn sleep<'a>(&'a self, duration: Duration) -> BoxFuture<'a, ()> {
            self.delays.lock().unwrap().push(duration);
            Box::pin(async {})
        }
    }

    struct PendingMetadataRetrySleeper;

    impl MetadataRetrySleeper for PendingMetadataRetrySleeper {
        fn sleep<'a>(&'a self, _duration: Duration) -> BoxFuture<'a, ()> {
            Box::pin(future::pending())
        }
    }

    fn fixture_source() -> (
        AgentsMirrorSource,
        Arc<FakeMetadataFetcher>,
        Arc<FakeClock>,
        Arc<FakeMetadataRetrySleeper>,
    ) {
        let fetcher = Arc::new(FakeMetadataFetcher::from_fixture());
        let clock = Arc::new(FakeClock::new(Instant::now()));
        let sleeper = Arc::new(FakeMetadataRetrySleeper::default());
        let source =
            AgentsMirrorSource::with_dependencies(fetcher.clone(), clock.clone(), sleeper.clone());
        (source, fetcher, clock, sleeper)
    }

    async fn resolve_without_cancellation(
        source: &dyn ReleaseSource,
        platform: DesktopPlatform,
        architecture: CpuArchitecture,
        cache_mode: CacheMode,
    ) -> Result<ReleaseDescriptor, InstallerError> {
        let cancellation = AtomicBool::new(false);
        source
            .resolve_latest(platform, architecture, cache_mode, &cancellation)
            .await
    }

    #[test]
    fn cache_is_partitioned_by_target_and_expires_at_five_minutes() {
        let now = Instant::now();
        let mut cache = ReleaseCache::default();
        cache.insert(RawTarget::WindowsX64, "x64", now);
        cache.insert(RawTarget::WindowsArm64, "arm64", now);

        assert_eq!(
            cache.get(RawTarget::WindowsX64, now + Duration::from_secs(299)),
            Some("x64")
        );
        assert_eq!(
            cache.get(RawTarget::WindowsArm64, now + Duration::from_secs(299)),
            Some("arm64")
        );
        assert_eq!(cache.get(RawTarget::MacosArm64, now), None);
        assert_eq!(
            cache.get(RawTarget::WindowsX64, now + RELEASE_CACHE_TTL),
            None
        );
    }

    #[test]
    fn metadata_fetcher_surface_is_limited_to_fixed_metadata_routes() {
        assert_eq!(
            MetadataEndpoint::Checksums.url(),
            "https://codexapp.agentsmirror.com/latest/checksums"
        );
        assert_eq!(
            MetadataEndpoint::Manifest.url(),
            "https://codexapp.agentsmirror.com/latest/manifest"
        );
    }

    #[tokio::test]
    async fn source_fetches_checksums_first_and_caches_per_platform_architecture() {
        let (source, fetcher, clock, _) = fixture_source();
        let source: &dyn ReleaseSource = &source;
        let first = resolve_without_cancellation(
            source,
            DesktopPlatform::Windows,
            CpuArchitecture::X86_64,
            CacheMode::UseCache,
        )
        .await
        .expect("valid fixture resolves");
        assert_eq!(first.download_endpoint, TrustedDownloadEndpoint::WinX64);
        assert_eq!(
            fetcher.calls(),
            vec![MetadataEndpoint::Checksums, MetadataEndpoint::Manifest]
        );

        let cached = resolve_without_cancellation(
            source,
            DesktopPlatform::Windows,
            CpuArchitecture::X86_64,
            CacheMode::UseCache,
        )
        .await
        .expect("cache hit");
        assert_eq!(cached.release_id(), first.release_id());
        assert_eq!(fetcher.calls().len(), 2);

        let arm64 = resolve_without_cancellation(
            source,
            DesktopPlatform::Windows,
            CpuArchitecture::Aarch64,
            CacheMode::UseCache,
        )
        .await
        .expect("different architecture resolves independently");
        assert_eq!(arm64.download_endpoint, TrustedDownloadEndpoint::WinArm64);
        assert_ne!(arm64.release_id(), first.release_id());
        assert_eq!(fetcher.calls().len(), 4);

        clock.advance(RELEASE_CACHE_TTL);
        let _ = resolve_without_cancellation(
            source,
            DesktopPlatform::Windows,
            CpuArchitecture::X86_64,
            CacheMode::UseCache,
        )
        .await
        .expect("expired cache is refreshed");
        assert_eq!(fetcher.calls().len(), 6);
    }

    #[tokio::test]
    async fn force_refresh_bypasses_cache_and_a_failure_preserves_last_success() {
        let (source, fetcher, _, _) = fixture_source();
        let first = resolve_without_cancellation(
            &source,
            DesktopPlatform::Macos,
            CpuArchitecture::Aarch64,
            CacheMode::UseCache,
        )
        .await
        .expect("initial metadata succeeds");

        let forced = resolve_without_cancellation(
            &source,
            DesktopPlatform::Macos,
            CpuArchitecture::Aarch64,
            CacheMode::ForceRefresh,
        )
        .await
        .expect("force bypasses a valid cache");
        assert_eq!(forced.release_id(), first.release_id());
        assert_eq!(fetcher.calls().len(), 4);

        fetcher.replace_manifest(b"not valid JSON".to_vec());
        let failure = resolve_without_cancellation(
            &source,
            DesktopPlatform::Macos,
            CpuArchitecture::Aarch64,
            CacheMode::ForceRefresh,
        )
        .await
        .expect_err("forced revalidation must observe invalid metadata");
        assert_eq!(failure.code(), InstallerErrorCode::ChecksumMismatch);
        assert_eq!(fetcher.calls().len(), 6);

        let cached = resolve_without_cancellation(
            &source,
            DesktopPlatform::Macos,
            CpuArchitecture::Aarch64,
            CacheMode::UseCache,
        )
        .await
        .expect("failed force refresh does not poison the last successful cache");
        assert_eq!(cached.release_id(), first.release_id());
        assert_eq!(fetcher.calls().len(), 6);
    }

    #[tokio::test]
    async fn unsupported_architecture_does_not_fetch_or_fallback() {
        let (source, fetcher, _, _) = fixture_source();
        let error = resolve_without_cancellation(
            &source,
            DesktopPlatform::Macos,
            CpuArchitecture::X86_64UnsupportedMac,
            CacheMode::UseCache,
        )
        .await
        .expect_err("Intel macOS has no V1 endpoint");

        assert_eq!(error.code(), InstallerErrorCode::ArchitectureUnsupported);
        assert!(fetcher.calls().is_empty());
    }

    #[tokio::test]
    async fn metadata_responses_are_collected_with_a_strict_one_mebibyte_cap() {
        let (source, fetcher, _, _) = fixture_source();
        fetcher.replace_checksums(FakeMetadataResponse::with_chunks(
            None,
            vec![Ok(Bytes::from(vec![b'x'; MAX_CHECKSUMS_BYTES + 1]))],
        ));

        let error = resolve_without_cancellation(
            &source,
            DesktopPlatform::Windows,
            CpuArchitecture::X86_64,
            CacheMode::ForceRefresh,
        )
        .await
        .expect_err("a streamed response above one mebibyte must be rejected before parsing");

        assert_eq!(error.code(), InstallerErrorCode::ReleaseMetadataInvalid);
        assert_eq!(
            error.to_dto().details.endpoint_kind.as_deref(),
            Some(MetadataEndpoint::Checksums.kind())
        );
        assert_eq!(fetcher.calls(), vec![MetadataEndpoint::Checksums]);
    }

    #[tokio::test]
    async fn retryable_metadata_failures_use_three_total_attempts_and_backoff() {
        let (source, fetcher, _, sleeper) = fixture_source();
        fetcher.queue_transport_error(
            MetadataEndpoint::Checksums,
            TransportError::retryable("first transient metadata failure"),
        );
        fetcher.queue_transport_error(
            MetadataEndpoint::Checksums,
            TransportError::retryable("second transient metadata failure"),
        );

        let resolved = resolve_without_cancellation(
            &source,
            DesktopPlatform::Windows,
            CpuArchitecture::X86_64,
            CacheMode::ForceRefresh,
        )
        .await
        .expect("the third checksums request should resolve from the fixture");

        assert_eq!(resolved.download_endpoint, TrustedDownloadEndpoint::WinX64);
        assert_eq!(
            fetcher.calls(),
            vec![
                MetadataEndpoint::Checksums,
                MetadataEndpoint::Checksums,
                MetadataEndpoint::Checksums,
                MetadataEndpoint::Manifest,
            ]
        );
        assert_eq!(
            sleeper.delays(),
            vec![Duration::from_secs(1), Duration::from_secs(3)]
        );
    }

    #[tokio::test]
    async fn retryable_metadata_body_failure_refetches_the_endpoint() {
        let (source, fetcher, _, sleeper) = fixture_source();
        fetcher.queue_response(
            MetadataEndpoint::Checksums,
            FakeMetadataResponse::with_chunks(
                None,
                vec![Err(TransportError::retryable("body stream interrupted"))],
            ),
        );

        let resolved = resolve_without_cancellation(
            &source,
            DesktopPlatform::Windows,
            CpuArchitecture::X86_64,
            CacheMode::ForceRefresh,
        )
        .await
        .expect("a retryable metadata body failure should refetch checksums");

        assert_eq!(resolved.download_endpoint, TrustedDownloadEndpoint::WinX64);
        assert_eq!(
            fetcher.calls(),
            vec![
                MetadataEndpoint::Checksums,
                MetadataEndpoint::Checksums,
                MetadataEndpoint::Manifest,
            ]
        );
        assert_eq!(sleeper.delays(), vec![Duration::from_secs(1)]);
    }

    #[tokio::test]
    async fn terminal_metadata_failure_does_not_retry_or_fetch_manifest() {
        let (source, fetcher, _, sleeper) = fixture_source();
        fetcher.queue_transport_error(
            MetadataEndpoint::Checksums,
            TransportError::non_retryable("metadata access denied"),
        );

        let error = resolve_without_cancellation(
            &source,
            DesktopPlatform::Windows,
            CpuArchitecture::X86_64,
            CacheMode::ForceRefresh,
        )
        .await
        .expect_err("a terminal metadata error must not be retried");

        assert_eq!(error.code(), InstallerErrorCode::SourceUnavailable);
        assert_eq!(error.to_dto().details.attempt, Some(1));
        assert_eq!(
            error.to_dto().details.max_attempts,
            Some(MAX_METADATA_ATTEMPTS)
        );
        assert_eq!(fetcher.calls(), vec![MetadataEndpoint::Checksums]);
        assert!(sleeper.delays().is_empty());
    }

    #[tokio::test]
    async fn metadata_redirect_policy_failure_is_terminal_and_keeps_its_stable_code() {
        let (source, fetcher, _, sleeper) = fixture_source();
        fetcher.queue_transport_error(
            MetadataEndpoint::Checksums,
            TransportError::redirect_rejected(
                "metadata redirect did not meet the installer policy",
            ),
        );

        let error = resolve_without_cancellation(
            &source,
            DesktopPlatform::Windows,
            CpuArchitecture::X86_64,
            CacheMode::ForceRefresh,
        )
        .await
        .expect_err("a rejected metadata redirect must not be retried");

        let dto = error.to_dto();
        assert_eq!(dto.code, InstallerErrorCode::RedirectRejected);
        assert!(!dto.retryable);
        assert_eq!(dto.suggested_action, SuggestedAction::OpenLogs);
        assert_eq!(dto.details.attempt, Some(1));
        assert_eq!(dto.details.max_attempts, Some(MAX_METADATA_ATTEMPTS));
        assert_eq!(fetcher.calls(), vec![MetadataEndpoint::Checksums]);
        assert!(sleeper.delays().is_empty());
    }

    #[tokio::test]
    async fn cancellation_stops_a_pending_metadata_fetch() {
        let fetcher = Arc::new(PendingMetadataFetcher::default());
        let source = AgentsMirrorSource::new(fetcher.clone());
        let cancellation = Arc::new(AtomicBool::new(false));
        let cancellation_for_task = cancellation.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            cancellation_for_task.store(true, Ordering::Release);
        });

        let error = source
            .resolve_latest(
                DesktopPlatform::Windows,
                CpuArchitecture::X86_64,
                CacheMode::ForceRefresh,
                cancellation.as_ref(),
            )
            .await
            .expect_err("cancellation must drop a pending metadata fetch");

        assert_eq!(error.code(), InstallerErrorCode::DownloadCancelled);
        assert_eq!(fetcher.calls(), vec![MetadataEndpoint::Checksums]);
    }

    #[tokio::test]
    async fn cancellation_stops_a_pending_metadata_retry_wait() {
        let fetcher = Arc::new(FakeMetadataFetcher::from_fixture());
        fetcher.queue_transport_error(
            MetadataEndpoint::Checksums,
            TransportError::retryable("metadata connection reset"),
        );
        let clock = Arc::new(FakeClock::new(Instant::now()));
        let source = AgentsMirrorSource::with_dependencies(
            fetcher.clone(),
            clock,
            Arc::new(PendingMetadataRetrySleeper),
        );
        let cancellation = Arc::new(AtomicBool::new(false));
        let cancellation_for_task = cancellation.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            cancellation_for_task.store(true, Ordering::Release);
        });

        let error = source
            .resolve_latest(
                DesktopPlatform::Windows,
                CpuArchitecture::X86_64,
                CacheMode::ForceRefresh,
                cancellation.as_ref(),
            )
            .await
            .expect_err("cancellation must stop retry backoff");

        assert_eq!(error.code(), InstallerErrorCode::DownloadCancelled);
        assert_eq!(fetcher.calls(), vec![MetadataEndpoint::Checksums]);
    }

    #[tokio::test]
    async fn already_cancelled_check_does_not_return_a_cached_descriptor() {
        let (source, fetcher, _, _) = fixture_source();
        let first = resolve_without_cancellation(
            &source,
            DesktopPlatform::Windows,
            CpuArchitecture::X86_64,
            CacheMode::UseCache,
        )
        .await
        .expect("fixture populates the cache");
        assert_eq!(first.download_endpoint, TrustedDownloadEndpoint::WinX64);

        let cancellation = AtomicBool::new(true);
        let error = source
            .resolve_latest(
                DesktopPlatform::Windows,
                CpuArchitecture::X86_64,
                CacheMode::UseCache,
                &cancellation,
            )
            .await
            .expect_err("an already-cancelled check must not consume the cache");

        assert_eq!(error.code(), InstallerErrorCode::DownloadCancelled);
        assert_eq!(fetcher.calls().len(), 2);
    }

    #[test]
    fn v5_fixture_yields_per_architecture_releases_without_remote_urls() {
        let windows =
            validate_release_metadata(VALID_CHECKSUMS, VALID_MANIFEST, RawTarget::WindowsX64)
                .expect("valid Windows x64 metadata");
        assert_eq!(windows.target, RawTarget::WindowsX64);
        assert_eq!(
            windows.artifact_file_name,
            "OpenAI.Codex_1.2.3.4_x64__fixture.Msix"
        );
        assert_eq!(windows.expected_sha256, "a".repeat(64));
        assert_eq!(windows.expected_size, 1_048_576);
        assert_eq!(windows.display_version, "1.2.3.4");
        assert_eq!(
            windows.platform_version,
            ValidatedPlatformVersion::WindowsMsix("1.2.3.4".to_owned())
        );

        let macos =
            validate_release_metadata(VALID_CHECKSUMS, VALID_MANIFEST, RawTarget::MacosArm64)
                .expect("valid macOS metadata");
        assert_eq!(macos.target, RawTarget::MacosArm64);
        assert_eq!(macos.artifact_file_name, "Codex-mac-arm64.dmg");
        assert_eq!(macos.expected_sha256, "c".repeat(64));
        assert_eq!(macos.expected_size, 593_861_752);
        assert_eq!(macos.display_version, "26.721.41059");
        assert_eq!(
            macos.platform_version,
            ValidatedPlatformVersion::MacBundle("5848".to_owned())
        );
    }

    #[test]
    fn macos_artifact_name_is_derived_from_the_validated_checksum_metadata() {
        let renamed_name = "OpenAI-Codex-26.721.41059-arm64.dmg";
        assert!(verify::validate_artifact_file_name(renamed_name, ArtifactKind::Dmg).is_ok());
        let mut manifest = manifest_value();
        let derived = manifest["derived"]["latestChecksums"]
            .as_object_mut()
            .expect("fixture derived checksums must be an object");
        derived.remove("Codex-mac-arm64.dmg");
        derived.insert(renamed_name.to_owned(), serde_json::json!("c".repeat(64)));
        let manifest = serde_json::to_vec(&manifest).unwrap();
        let checksums =
            checksums_for_with_macos_artifacts(&manifest, &[(renamed_name, &"c".repeat(64))]);

        let release = validate_release_metadata(&checksums, &manifest, RawTarget::MacosArm64)
            .expect("a branch-pinned, cross-checked renamed DMG must stay usable");

        assert_eq!(release.artifact_file_name, renamed_name);
        assert_eq!(release.expected_sha256, "c".repeat(64));
    }

    #[test]
    fn macos_artifact_name_fails_closed_when_the_branch_checksum_matches_multiple_dmgs() {
        let mut manifest = manifest_value();
        let derived = manifest["derived"]["latestChecksums"]
            .as_object_mut()
            .expect("fixture derived checksums must be an object");
        derived.remove("Codex-mac-arm64.dmg");
        derived.insert(
            "Codex-arm64-primary.dmg".to_owned(),
            serde_json::json!("c".repeat(64)),
        );
        derived.insert(
            "Codex-arm64-secondary.dmg".to_owned(),
            serde_json::json!("c".repeat(64)),
        );
        let manifest = serde_json::to_vec(&manifest).unwrap();
        let checksums = checksums_for_with_macos_artifacts(
            &manifest,
            &[
                ("Codex-arm64-primary.dmg", &"c".repeat(64)),
                ("Codex-arm64-secondary.dmg", &"c".repeat(64)),
            ],
        );

        assert_eq!(
            validate_release_metadata(&checksums, &manifest, RawTarget::MacosArm64),
            Err(SourceValidationFailure::MetadataInvalid(
                "macOS ARM64 artifact filename is ambiguous"
            ))
        );
    }

    #[test]
    fn manifest_hash_failure_happens_before_json_parsing() {
        let invalid_json = b"not JSON at all";
        let checksums = format!("{}  release-manifest.json\n", "0".repeat(64));

        assert_eq!(
            validate_release_metadata(checksums.as_bytes(), invalid_json, RawTarget::WindowsX64),
            Err(SourceValidationFailure::ManifestChecksumMismatch)
        );
    }

    #[test]
    fn manifest_size_is_bounded_before_json_parsing() {
        let oversized = vec![b' '; MAX_MANIFEST_BYTES + 1];
        let checksums = format!(
            "{}  release-manifest.json\n",
            verify::sha256_hex(&oversized)
        );

        assert_eq!(
            validate_release_metadata(checksums.as_bytes(), &oversized, RawTarget::WindowsX64),
            Err(SourceValidationFailure::MetadataInvalid(
                "release manifest response is too large"
            ))
        );
    }

    #[test]
    fn schema_drift_and_required_branch_fields_fail_closed() {
        let mut schema_drift = manifest_value();
        schema_drift["schemaVersion"] = serde_json::json!(6);
        let schema_drift = serde_json::to_vec(&schema_drift).unwrap();
        assert!(matches!(
            validate_release_metadata(
                &checksums_for(&schema_drift),
                &schema_drift,
                RawTarget::WindowsX64
            ),
            Err(SourceValidationFailure::MetadataInvalid(_))
        ));

        let mut zero_size = manifest_value();
        zero_size["sources"]["windows"]["architectures"]["x64"]["contentLength"] =
            serde_json::json!(0);
        let zero_size = serde_json::to_vec(&zero_size).unwrap();
        assert!(matches!(
            validate_release_metadata(
                &checksums_for(&zero_size),
                &zero_size,
                RawTarget::WindowsX64
            ),
            Err(SourceValidationFailure::MetadataInvalid(_))
        ));

        let mut wrong_architecture = manifest_value();
        wrong_architecture["sources"]["windows"]["architectures"]["x64"]["architecture"] =
            serde_json::json!("arm64");
        let wrong_architecture = serde_json::to_vec(&wrong_architecture).unwrap();
        assert!(matches!(
            validate_release_metadata(
                &checksums_for(&wrong_architecture),
                &wrong_architecture,
                RawTarget::WindowsX64
            ),
            Err(SourceValidationFailure::MetadataInvalid(_))
        ));

        let mut unsafe_file_name = manifest_value();
        unsafe_file_name["sources"]["windows"]["architectures"]["x64"]["packageMoniker"] =
            serde_json::json!("../evil");
        let unsafe_file_name = serde_json::to_vec(&unsafe_file_name).unwrap();
        assert!(matches!(
            validate_release_metadata(
                &checksums_for(&unsafe_file_name),
                &unsafe_file_name,
                RawTarget::WindowsX64
            ),
            Err(SourceValidationFailure::MetadataInvalid(_))
        ));
    }

    #[test]
    fn checksum_parser_rejects_duplicate_unsafe_and_invalid_entries() {
        let duplicate = format!(
            "{}  package.msix\n{}  package.msix\n",
            "a".repeat(64),
            "a".repeat(64)
        );
        assert!(matches!(
            parse_checksums(duplicate.as_bytes()),
            Err(SourceValidationFailure::MetadataInvalid(_))
        ));

        let traversal = format!("{}  ../package.msix\n", "a".repeat(64));
        assert!(matches!(
            parse_checksums(traversal.as_bytes()),
            Err(SourceValidationFailure::MetadataInvalid(_))
        ));

        assert!(matches!(
            parse_checksums(b"not a checksum line\n"),
            Err(SourceValidationFailure::MetadataInvalid(_))
        ));
    }

    #[test]
    fn release_requires_each_checksum_source_to_agree() {
        let mut missing_artifact = VALID_CHECKSUMS.to_vec();
        let marker = b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa  OpenAI.Codex_1.2.3.4_x64__fixture.Msix\n";
        let start = missing_artifact
            .windows(marker.len())
            .position(|window| window == marker)
            .expect("fixture line");
        missing_artifact.drain(start..start + marker.len());
        assert_eq!(
            validate_release_metadata(&missing_artifact, VALID_MANIFEST, RawTarget::WindowsX64),
            Err(SourceValidationFailure::ChecksumMissing(
                "artifact checksum"
            ))
        );

        let mut branch_mismatch = manifest_value();
        branch_mismatch["sources"]["macos"]["arm64"]["sha256"] = serde_json::json!("d".repeat(64));
        let branch_mismatch = serde_json::to_vec(&branch_mismatch).unwrap();
        assert_eq!(
            validate_release_metadata(
                &checksums_for(&branch_mismatch),
                &branch_mismatch,
                RawTarget::MacosArm64
            ),
            Err(SourceValidationFailure::ArtifactChecksumMismatch)
        );

        let mut malformed_branch_checksum = manifest_value();
        malformed_branch_checksum["sources"]["macos"]["arm64"]["sha256"] =
            serde_json::json!(format!(" {}", "c".repeat(64)));
        let malformed_branch_checksum = serde_json::to_vec(&malformed_branch_checksum).unwrap();
        assert!(matches!(
            validate_release_metadata(
                &checksums_for(&malformed_branch_checksum),
                &malformed_branch_checksum,
                RawTarget::MacosArm64
            ),
            Err(SourceValidationFailure::MetadataInvalid(_))
        ));
    }

    #[test]
    fn unavailable_or_missing_current_branch_is_not_a_cross_architecture_fallback() {
        let mut unavailable = manifest_value();
        unavailable["sources"]["windows"]["architectures"]["x64"]["downloadable"] =
            serde_json::json!(false);
        unavailable["sources"]["windows"]["architectures"]["x64"]["status"] =
            serde_json::json!("catalog-only");
        let unavailable = serde_json::to_vec(&unavailable).unwrap();
        assert_eq!(
            validate_release_metadata(
                &checksums_for(&unavailable),
                &unavailable,
                RawTarget::WindowsX64
            ),
            Err(SourceValidationFailure::ReleaseNotAvailable)
        );

        let mut missing = manifest_value();
        missing["sources"]["windows"]["architectures"]
            .as_object_mut()
            .unwrap()
            .remove("x64");
        let missing = serde_json::to_vec(&missing).unwrap();
        assert_eq!(
            validate_release_metadata(&checksums_for(&missing), &missing, RawTarget::WindowsX64),
            Err(SourceValidationFailure::ReleaseNotAvailable)
        );
    }
}

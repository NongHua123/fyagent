//! 下载器的无副作用安全策略。
//!
//! 真正的 HTTP 与文件系统实现通过可注入 adapter 接入。这里先把不能因
//! adapter 差异而改变的 redirect、重试和诊断规则集中起来，避免 source、
//! downloader 与平台层各自解释 URL。

use std::{
    fmt,
    fs::OpenOptions,
    future::Future,
    io::Write,
    path::{Path, PathBuf},
    pin::Pin,
    time::{Duration, Instant},
};

use bytes::Bytes;
use futures::{Stream, StreamExt};
use thiserror::Error;
use url::Url;

use super::{
    cancellation::{cancellation_error, race_with_cancellation, Cancellation},
    error::{InstallerError, InstallerErrorCode},
    temp::JobTempDir,
    types::{ProgressPhase, ReleaseDescriptor, TrustedDownloadEndpoint},
    verify::{self, ArtifactKind},
};

pub const MAX_REDIRECTS: usize = 5;
pub const MAX_DOWNLOAD_ATTEMPTS: u8 = 3;
const MAX_RETRY_AFTER_SECS: u64 = 30;
const INSTALLER_CONNECT_TIMEOUT: Duration = Duration::from_secs(30);
const INSTALLER_READ_IDLE_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TransportErrorKind {
    Other,
    Timeout,
    RedirectRejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryDisposition {
    Retry,
    DoNotRetry,
}

/// A transport error is deliberately narrower than an installer error. The
/// downloader decides whether it is retryable and maps it to the stable IPC
/// error only after attempts have been exhausted.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[error("installer transport error")]
pub struct TransportError {
    retryable: bool,
    kind: TransportErrorKind,
    diagnostic: String,
}

impl TransportError {
    pub fn retryable(diagnostic: impl Into<String>) -> Self {
        Self {
            retryable: true,
            kind: TransportErrorKind::Other,
            diagnostic: diagnostic.into(),
        }
    }

    pub fn timeout(diagnostic: impl Into<String>) -> Self {
        Self {
            retryable: true,
            kind: TransportErrorKind::Timeout,
            diagnostic: diagnostic.into(),
        }
    }

    pub fn non_retryable(diagnostic: impl Into<String>) -> Self {
        Self {
            retryable: false,
            kind: TransportErrorKind::Other,
            diagnostic: diagnostic.into(),
        }
    }

    /// Preserve a local redirect-policy rejection across the narrow transport
    /// boundary so metadata resolution can keep its stable installer error.
    /// It is deliberately non-retryable: retrying cannot make an insecure or
    /// malformed redirect acceptable.
    pub(crate) fn redirect_rejected(diagnostic: impl Into<String>) -> Self {
        Self {
            retryable: false,
            kind: TransportErrorKind::RedirectRejected,
            diagnostic: diagnostic.into(),
        }
    }

    pub fn is_retryable(&self) -> bool {
        self.retryable
    }

    pub fn is_timeout(&self) -> bool {
        self.kind == TransportErrorKind::Timeout
    }

    pub(crate) fn is_redirect_rejected(&self) -> bool {
        self.kind == TransportErrorKind::RedirectRejected
    }

    /// This is for bounded internal diagnostics only. It is never an IPC
    /// message, because a transport implementation may include network text.
    pub(crate) fn diagnostic(&self) -> &str {
        &self.diagnostic
    }
}

pub type BodyStream = Pin<Box<dyn Stream<Item = Result<Bytes, TransportError>> + Send>>;
pub type TransportFuture<'a> =
    Pin<Box<dyn Future<Output = Result<TransportResponse, TransportError>> + Send + 'a>>;

pub struct TransportResponse {
    pub status: u16,
    pub location: Option<String>,
    pub content_length: Option<u64>,
    pub retry_after: Option<String>,
    pub body: BodyStream,
}

/// A client with automatic redirect disabled. Keeping this object-safe makes
/// the service testable without an HTTP server or a production package.
pub trait HttpTransport: Send + Sync {
    fn get(&self, url: Url) -> TransportFuture<'_>;
}

/// Parameters copied from the existing proxy configuration into a fresh,
/// credential-free installer client. The actual global Client is intentionally
/// not accepted here, because it can have automatic redirects or default
/// credential headers that would bypass the installer trust boundary.
pub(crate) struct InstallerHttpClientOptions {
    proxy_url: Option<Url>,
    user_agent: String,
    total_timeout: Option<Duration>,
    use_system_proxy: bool,
}

impl InstallerHttpClientOptions {
    pub(crate) fn for_download(proxy_url: Option<Url>, user_agent: impl Into<String>) -> Self {
        Self {
            proxy_url,
            user_agent: user_agent.into(),
            total_timeout: None,
            use_system_proxy: true,
        }
    }

    pub(crate) fn for_metadata(proxy_url: Option<Url>, user_agent: impl Into<String>) -> Self {
        Self {
            proxy_url,
            user_agent: user_agent.into(),
            total_timeout: Some(Duration::from_secs(30)),
            use_system_proxy: true,
        }
    }

    /// Avoids a known self-referential system proxy while preserving explicit
    /// proxy support for the scoped installer client.
    pub(crate) fn without_system_proxy(mut self) -> Self {
        self.use_system_proxy = false;
        self
    }
}

/// Build the sole production HTTP client shape permitted for installer
/// traffic. It has no injected default headers, disables all automatic content
/// decoding, and leaves redirects to the local manual redirect policy.
pub(crate) fn build_installer_http_client(
    options: InstallerHttpClientOptions,
) -> Result<reqwest::Client, InstallerError> {
    let mut builder = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(INSTALLER_CONNECT_TIMEOUT)
        .read_timeout(INSTALLER_READ_IDLE_TIMEOUT)
        .no_gzip()
        .no_brotli()
        .no_deflate()
        .user_agent(options.user_agent);
    if let Some(proxy_url) = options.proxy_url {
        let proxy = reqwest::Proxy::all(proxy_url.as_str()).map_err(|_| {
            InstallerError::new(InstallerErrorCode::SourceUnavailable)
                .with_diagnostic_message("installer proxy configuration is invalid")
        })?;
        builder = builder.proxy(proxy);
    }
    if !options.use_system_proxy {
        builder = builder.no_proxy();
    }
    if let Some(timeout) = options.total_timeout {
        builder = builder.timeout(timeout);
    }
    builder.build().map_err(|_| {
        InstallerError::new(InstallerErrorCode::SourceUnavailable)
            .with_diagnostic_message("installer HTTP client could not be created")
    })
}

/// Production adapter backed only by the installer client builder above.
pub struct ReqwestInstallerTransport {
    client: reqwest::Client,
}

impl ReqwestInstallerTransport {
    pub(crate) fn new(options: InstallerHttpClientOptions) -> Result<Self, InstallerError> {
        Ok(Self {
            client: build_installer_http_client(options)?,
        })
    }
}

impl HttpTransport for ReqwestInstallerTransport {
    fn get(&self, url: Url) -> TransportFuture<'_> {
        let client = self.client.clone();
        Box::pin(async move {
            let response = client
                .get(url)
                .header(reqwest::header::ACCEPT_ENCODING, "identity")
                .send()
                .await
                .map_err(transport_error_from_reqwest)?;

            let status = response.status().as_u16();
            let headers = response.headers();
            let location = headers
                .get(reqwest::header::LOCATION)
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned);
            let content_length = response.content_length();
            let retry_after = headers
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned);
            let body = Box::pin(
                response
                    .bytes_stream()
                    .map(|result| result.map_err(transport_error_from_reqwest)),
            );

            Ok(TransportResponse {
                status,
                location,
                content_length,
                retry_after,
                body,
            })
        })
    }
}

fn transport_error_from_reqwest(error: reqwest::Error) -> TransportError {
    let diagnostic = error.to_string();
    if error.is_timeout() {
        TransportError::timeout(diagnostic)
    } else if error.is_connect() || error.is_request() || error.is_body() {
        TransportError::retryable(diagnostic)
    } else {
        TransportError::non_retryable(diagnostic)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DownloadProgressUpdate {
    pub phase: ProgressPhase,
    pub attempt: u8,
    pub max_attempts: u8,
    pub completed_bytes: u64,
    pub total_bytes: u64,
}

pub trait DownloadProgressSink: Send + Sync {
    fn emit(&self, update: DownloadProgressUpdate);
}

impl<F> DownloadProgressSink for F
where
    F: Fn(DownloadProgressUpdate) + Send + Sync,
{
    fn emit(&self, update: DownloadProgressUpdate) {
        self(update);
    }
}

/// Opaque evidence that one fixed job artifact completed the downloader's
/// integrity checks.
///
/// In addition to the final path, retain only the canonical job-root capability
/// needed to reopen that directory later. Platform adapters must revalidate
/// this evidence immediately before every package parser, mount, or deployment
/// consumes the file; retaining a bare path would leave a replacement window
/// between download verification and platform installation.
#[derive(Clone, PartialEq, Eq)]
pub struct DownloadedArtifact {
    path: PathBuf,
    size: u64,
    job_root: PathBuf,
    job_id: String,
    artifact_kind: ArtifactKind,
}

impl fmt::Debug for DownloadedArtifact {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DownloadedArtifact")
            .field("path", &"<downloaded-artifact>")
            .field("size", &self.size)
            .finish()
    }
}

impl DownloadedArtifact {
    #[cfg_attr(not(any(target_os = "windows", target_os = "macos")), allow(dead_code))]
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    /// Reopens the downloader-owned UUID directory and repeats the fixed-file
    /// and descriptor integrity gates. `JobTempDir::open_existing` restores
    /// canonical root/direct-child containment, while
    /// `validate_existing_artifact` rejects links, reparse points, and
    /// non-regular files before SHA-256 is calculated.
    #[cfg_attr(not(any(target_os = "windows", target_os = "macos")), allow(dead_code))]
    pub(crate) fn revalidate_against(
        &self,
        release: &ReleaseDescriptor,
    ) -> Result<(), InstallerError> {
        let expected_kind = artifact_kind_for_endpoint(release.download_endpoint)?;
        if self.artifact_kind != expected_kind || self.size != release.expected_size {
            return Err(
                InstallerError::new(InstallerErrorCode::PackageIdentityMismatch)
                    .with_diagnostic_message(
                        "downloaded artifact evidence does not match the locked release descriptor",
                    ),
            );
        }

        let job_directory = JobTempDir::open_existing(&self.job_root, &self.job_id)?;
        if job_directory.final_path(self.artifact_kind) != self.path {
            return Err(
                InstallerError::new(InstallerErrorCode::PackageIdentityMismatch)
                    .with_diagnostic_message(
                        "downloaded artifact path no longer matches the locked job capability",
                    ),
            );
        }
        job_directory.validate_existing_artifact(&self.path)?;
        verify::verify_file(&self.path, release.expected_size, &release.expected_sha256)
    }

    fn from_completed_download(
        job_directory: &JobTempDir,
        release: &ReleaseDescriptor,
        size: u64,
    ) -> Result<Self, InstallerError> {
        let artifact_kind = artifact_kind_for_endpoint(release.download_endpoint)?;
        let path = job_directory.final_path(artifact_kind);
        job_directory.validate_existing_artifact(&path)?;
        let job_root = job_directory.path().parent().ok_or_else(|| {
            InstallerError::new(InstallerErrorCode::InternalError)
                .with_diagnostic_message("installer job directory has no trusted root")
        })?;
        let job_id = job_directory
            .path()
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| {
                InstallerError::new(InstallerErrorCode::InternalError)
                    .with_diagnostic_message("installer job directory has no canonical identifier")
            })?;

        Ok(Self {
            path,
            size,
            job_root: job_root.to_path_buf(),
            job_id: job_id.to_owned(),
            artifact_kind,
        })
    }

    #[cfg(test)]
    pub(crate) fn from_test_file(
        job_directory: &JobTempDir,
        release: &ReleaseDescriptor,
    ) -> Result<Self, InstallerError> {
        let artifact_kind = artifact_kind_for_endpoint(release.download_endpoint)?;
        let path = job_directory.final_path(artifact_kind);
        let size = std::fs::metadata(&path)
            .map_err(|_| {
                InstallerError::new(InstallerErrorCode::InternalError)
                    .with_diagnostic_message("test installer artifact could not be inspected")
            })?
            .len();
        let artifact = Self::from_completed_download(job_directory, release, size)?;
        artifact.revalidate_against(release)?;
        Ok(artifact)
    }
}

#[derive(Debug)]
struct DownloadAttemptError {
    error: InstallerError,
    retryable: bool,
    retry_after: Option<Duration>,
}

impl DownloadAttemptError {
    fn retryable(error: InstallerError, retry_after: Option<Duration>) -> Self {
        Self {
            error,
            retryable: true,
            retry_after,
        }
    }

    fn terminal(error: InstallerError) -> Self {
        Self {
            error,
            retryable: false,
            retry_after: None,
        }
    }
}

/// Download one already-validated descriptor into a job-specific directory.
///
/// The directory comes from the service's UUID-derived temporary root. The
/// remote artifact name is validated by source but is never joined into this
/// path; only fixed local names from ArtifactKind are used.
pub(crate) async fn download_release(
    transport: &dyn HttpTransport,
    release: &ReleaseDescriptor,
    job_directory: &JobTempDir,
    cancellation: &dyn Cancellation,
    progress: &dyn DownloadProgressSink,
) -> Result<DownloadedArtifact, InstallerError> {
    let endpoint = release.download_endpoint;
    let initial_url = Url::parse(endpoint.url()).map_err(|_| {
        InstallerError::new(InstallerErrorCode::InternalError)
            .with_diagnostic_message("built-in download endpoint is invalid")
            .with_endpoint_kind(endpoint.kind())
    })?;
    let artifact_kind = artifact_kind_for_endpoint(endpoint)?;
    let part_path = job_directory.part_path(artifact_kind);
    let final_path = job_directory.final_path(artifact_kind);
    job_directory
        .validate_artifact_path(&part_path)
        .and_then(|_| job_directory.validate_artifact_path(&final_path))
        .and_then(|_| ensure_path_absent(&part_path))
        .and_then(|_| ensure_path_absent(&final_path))
        .map_err(|error| error.with_endpoint_kind(endpoint.kind()))?;

    for attempt in 1..=MAX_DOWNLOAD_ATTEMPTS {
        if cancellation.is_cancelled() {
            remove_file_if_exists(job_directory, &part_path);
            return Err(cancelled_error()
                .with_endpoint_kind(endpoint.kind())
                .with_attempt(attempt, MAX_DOWNLOAD_ATTEMPTS));
        }

        progress.emit(DownloadProgressUpdate {
            phase: ProgressPhase::Download,
            attempt,
            max_attempts: MAX_DOWNLOAD_ATTEMPTS,
            completed_bytes: 0,
            total_bytes: release.expected_size,
        });

        match download_attempt(
            transport,
            &initial_url,
            release,
            job_directory,
            &part_path,
            &final_path,
            cancellation,
            progress,
            attempt,
        )
        .await
        {
            Ok(artifact) => return Ok(artifact),
            Err(attempt_error) if attempt_error.retryable && attempt < MAX_DOWNLOAD_ATTEMPTS => {
                remove_file_if_exists(job_directory, &part_path);
                remove_file_if_exists(job_directory, &final_path);
                let delay = attempt_error
                    .retry_after
                    .unwrap_or_else(|| retry_delay_after(attempt));
                if let Err(error) = sleep_with_cancellation(delay, cancellation).await {
                    remove_file_if_exists(job_directory, &part_path);
                    remove_file_if_exists(job_directory, &final_path);
                    return Err(error
                        .with_endpoint_kind(endpoint.kind())
                        .with_attempt(attempt, MAX_DOWNLOAD_ATTEMPTS));
                }
            }
            Err(attempt_error) => {
                remove_file_if_exists(job_directory, &part_path);
                remove_file_if_exists(job_directory, &final_path);
                return Err(attempt_error
                    .error
                    .with_endpoint_kind(endpoint.kind())
                    .with_attempt(attempt, MAX_DOWNLOAD_ATTEMPTS));
            }
        }
    }

    Err(InstallerError::new(InstallerErrorCode::InternalError)
        .with_diagnostic_message("download retry loop exited unexpectedly")
        .with_endpoint_kind(endpoint.kind()))
}

#[allow(clippy::too_many_arguments)]
async fn download_attempt(
    transport: &dyn HttpTransport,
    initial_url: &Url,
    release: &ReleaseDescriptor,
    job_directory: &JobTempDir,
    part_path: &Path,
    final_path: &Path,
    cancellation: &dyn Cancellation,
    progress: &dyn DownloadProgressSink,
    attempt: u8,
) -> Result<DownloadedArtifact, DownloadAttemptError> {
    let response = match get_with_redirects(transport, initial_url.clone(), cancellation).await {
        Ok(response) => response,
        Err(RedirectRequestError::Cancelled) => {
            return Err(DownloadAttemptError::terminal(cancelled_error()));
        }
        Err(RedirectRequestError::Redirect(_)) => {
            return Err(DownloadAttemptError::terminal(
                InstallerError::new(InstallerErrorCode::RedirectRejected)
                    .with_diagnostic_message("download redirect did not meet the installer policy"),
            ));
        }
        Err(RedirectRequestError::Transport(error)) => {
            return Err(transport_attempt_error(error));
        }
    };

    if !(200..=299).contains(&response.status) {
        let retry_after = bounded_retry_after(response.retry_after.as_deref());
        let error = InstallerError::new(InstallerErrorCode::DownloadFailed)
            .with_http_status(response.status)
            .with_diagnostic_message("download endpoint returned a non-success response");
        return match retry_disposition_for_status(response.status) {
            RetryDisposition::Retry => Err(DownloadAttemptError::retryable(error, retry_after)),
            RetryDisposition::DoNotRetry => Err(DownloadAttemptError::terminal(error)),
        };
    }

    if response
        .content_length
        .is_some_and(|size| size != release.expected_size)
    {
        return Err(DownloadAttemptError::terminal(
            InstallerError::new(InstallerErrorCode::DownloadFailed)
                .with_diagnostic_message("download content length did not match release metadata"),
        ));
    }

    job_directory
        .validate_artifact_path(part_path)
        .map_err(DownloadAttemptError::terminal)?;
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(part_path)
        .map_err(|_| {
            DownloadAttemptError::terminal(
                InstallerError::new(InstallerErrorCode::DownloadFailed)
                    .with_diagnostic_message("installer partial file could not be created"),
            )
        })?;
    let mut body = response.body;
    let mut completed_bytes = 0_u64;
    let mut last_progress_emit = Instant::now();
    let mut last_progress_bytes = 0_u64;

    while let Some(chunk) = body.next().await {
        if cancellation.is_cancelled() {
            return Err(DownloadAttemptError::terminal(cancelled_error()));
        }

        let chunk = chunk.map_err(transport_attempt_error)?;
        completed_bytes = completed_bytes
            .checked_add(chunk.len() as u64)
            .ok_or_else(|| {
                DownloadAttemptError::terminal(
                    InstallerError::new(InstallerErrorCode::DownloadFailed)
                        .with_diagnostic_message("download exceeded supported size range"),
                )
            })?;
        if completed_bytes > release.expected_size {
            return Err(DownloadAttemptError::terminal(
                InstallerError::new(InstallerErrorCode::DownloadFailed)
                    .with_diagnostic_message("downloaded bytes exceeded release metadata"),
            ));
        }

        output.write_all(&chunk).map_err(|_| {
            DownloadAttemptError::terminal(
                InstallerError::new(InstallerErrorCode::DownloadFailed)
                    .with_diagnostic_message("installer partial file could not be written"),
            )
        })?;

        let now = Instant::now();
        if completed_bytes == release.expected_size
            || completed_bytes.saturating_sub(last_progress_bytes) >= 1024 * 1024
            || now.duration_since(last_progress_emit) >= Duration::from_millis(100)
        {
            progress.emit(DownloadProgressUpdate {
                phase: ProgressPhase::Download,
                attempt,
                max_attempts: MAX_DOWNLOAD_ATTEMPTS,
                completed_bytes,
                total_bytes: release.expected_size,
            });
            last_progress_emit = now;
            last_progress_bytes = completed_bytes;
        }
    }

    if cancellation.is_cancelled() {
        return Err(DownloadAttemptError::terminal(cancelled_error()));
    }
    if completed_bytes != release.expected_size {
        return Err(DownloadAttemptError::terminal(
            InstallerError::new(InstallerErrorCode::DownloadFailed)
                .with_diagnostic_message("downloaded byte count did not match release metadata"),
        ));
    }

    output.flush().map_err(|_| {
        DownloadAttemptError::terminal(
            InstallerError::new(InstallerErrorCode::DownloadFailed)
                .with_diagnostic_message("installer partial file could not be flushed"),
        )
    })?;
    output.sync_all().map_err(|_| {
        DownloadAttemptError::terminal(
            InstallerError::new(InstallerErrorCode::DownloadFailed)
                .with_diagnostic_message("installer partial file could not be synchronized"),
        )
    })?;
    drop(output);

    job_directory
        .validate_artifact_path(part_path)
        .and_then(|_| job_directory.validate_artifact_path(final_path))
        .and_then(|_| ensure_path_absent(final_path))
        .map_err(DownloadAttemptError::terminal)?;
    std::fs::rename(part_path, final_path).map_err(|_| {
        DownloadAttemptError::terminal(
            InstallerError::new(InstallerErrorCode::DownloadFailed)
                .with_diagnostic_message("installer partial file could not be finalized"),
        )
    })?;

    progress.emit(DownloadProgressUpdate {
        phase: ProgressPhase::Verification,
        attempt,
        max_attempts: MAX_DOWNLOAD_ATTEMPTS,
        completed_bytes,
        total_bytes: release.expected_size,
    });
    if cancellation.is_cancelled() {
        return Err(DownloadAttemptError::terminal(cancelled_error()));
    }
    job_directory
        .validate_existing_artifact(final_path)
        .map_err(DownloadAttemptError::terminal)?;
    verify::verify_file(final_path, release.expected_size, &release.expected_sha256)
        .map_err(DownloadAttemptError::terminal)?;
    progress.emit(DownloadProgressUpdate {
        phase: ProgressPhase::Verification,
        attempt,
        max_attempts: MAX_DOWNLOAD_ATTEMPTS,
        completed_bytes,
        total_bytes: release.expected_size,
    });
    DownloadedArtifact::from_completed_download(job_directory, release, completed_bytes)
        .map_err(DownloadAttemptError::terminal)
}

fn artifact_kind_for_endpoint(
    endpoint: TrustedDownloadEndpoint,
) -> Result<ArtifactKind, InstallerError> {
    match endpoint {
        TrustedDownloadEndpoint::WinX64 | TrustedDownloadEndpoint::WinArm64 => {
            Ok(ArtifactKind::Msix)
        }
        TrustedDownloadEndpoint::MacArm64 => Ok(ArtifactKind::Dmg),
        TrustedDownloadEndpoint::Manifest | TrustedDownloadEndpoint::Checksums => {
            Err(InstallerError::new(InstallerErrorCode::InternalError)
                .with_diagnostic_message("metadata endpoint cannot download an installer artifact"))
        }
    }
}

fn transport_attempt_error(error: TransportError) -> DownloadAttemptError {
    let code = if error.is_timeout() {
        InstallerErrorCode::DownloadTimeout
    } else {
        InstallerErrorCode::DownloadFailed
    };
    let installer_error = InstallerError::new(code).with_diagnostic_message(error.diagnostic());
    if error.is_retryable() {
        DownloadAttemptError::retryable(installer_error, None)
    } else {
        DownloadAttemptError::terminal(installer_error)
    }
}

fn retry_delay_after(failed_attempt: u8) -> Duration {
    match failed_attempt {
        1 => Duration::from_secs(1),
        _ => Duration::from_secs(3),
    }
}

async fn sleep_with_cancellation(
    delay: Duration,
    cancellation: &dyn Cancellation,
) -> Result<(), InstallerError> {
    match race_with_cancellation(tokio::time::sleep(delay), cancellation).await {
        Ok(()) => Ok(()),
        Err(_) => Err(cancelled_error()),
    }
}

fn ensure_path_absent(path: &Path) -> Result<(), InstallerError> {
    match std::fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Ok(_) => Err(InstallerError::new(InstallerErrorCode::InternalError)
            .with_diagnostic_message("installer temporary file unexpectedly already exists")),
        Err(_) => Err(InstallerError::new(InstallerErrorCode::InternalError)
            .with_diagnostic_message("installer temporary file could not be inspected")),
    }
}

fn remove_file_if_exists(job_directory: &JobTempDir, path: &Path) {
    if job_directory.validate_artifact_path(path).is_err() {
        return;
    }
    match std::fs::remove_file(path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => {}
    }
}

fn cancelled_error() -> InstallerError {
    cancellation_error()
}

#[derive(Debug, Error)]
pub enum RedirectRequestError {
    #[error("installer request was cancelled")]
    Cancelled,
    #[error(transparent)]
    Redirect(#[from] RedirectPolicyError),
    #[error("installer transport request failed")]
    Transport(TransportError),
}

/// Request an endpoint with the only redirect behavior permitted to the
/// installer. The returned body remains unread so a caller can stream it to a
/// job-local `.part` file and account for every byte.
pub async fn get_with_redirects(
    transport: &dyn HttpTransport,
    initial: Url,
    cancellation: &dyn Cancellation,
) -> Result<TransportResponse, RedirectRequestError> {
    validate_initial_endpoint(&initial)?;

    let mut current = initial;
    let mut followed_redirects = 0;

    loop {
        let response = race_with_cancellation(transport.get(current.clone()), cancellation)
            .await
            .map_err(|_| RedirectRequestError::Cancelled)?
            .map_err(RedirectRequestError::Transport)?;

        if !(300..=399).contains(&response.status) {
            return Ok(response);
        }

        let location = response
            .location
            .clone()
            .ok_or(RedirectPolicyError::InvalidLocation)?;
        drop(response);

        current = resolve_redirect(&current, &location, followed_redirects)?;
        followed_redirects += 1;
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RedirectPolicyError {
    #[error("redirect count exceeds the installer policy")]
    TooManyRedirects,
    #[error("redirect URL must use HTTPS")]
    InsecureScheme,
    #[error("redirect URL must not include user information")]
    UserInfo,
    #[error("redirect URL is invalid")]
    InvalidLocation,
}

/// Validate the initial fixed endpoint before an HTTP adapter sees it.
pub fn validate_initial_endpoint(url: &Url) -> Result<(), RedirectPolicyError> {
    validate_https_url(url)
}

/// Resolve one redirect without carrying any ambient request state.
///
/// The caller owns the count of already-followed redirects. Five redirects are
/// allowed; a sixth location is rejected before it can become a request.
pub fn resolve_redirect(
    current: &Url,
    location: &str,
    followed_redirects: usize,
) -> Result<Url, RedirectPolicyError> {
    if followed_redirects >= MAX_REDIRECTS {
        return Err(RedirectPolicyError::TooManyRedirects);
    }

    let next = current
        .join(location)
        .map_err(|_| RedirectPolicyError::InvalidLocation)?;
    validate_https_url(&next)?;
    Ok(next)
}

/// Return a URL representation that is safe to place in installer diagnostics.
/// Query, fragment and user information are deliberately omitted.
pub fn diagnostic_url(url: &Url) -> String {
    let host = url.host_str().unwrap_or("?");
    let port = url
        .port()
        .map(|value| format!(":{value}"))
        .unwrap_or_default();
    format!("{}://{host}{port}{}", url.scheme(), url.path())
}

/// Retry only network/transient HTTP categories. Integrity, policy and caller
/// cancellation failures must never be retried by the downloader.
pub fn retry_disposition_for_status(status: u16) -> RetryDisposition {
    match status {
        408 | 429 | 500..=599 => RetryDisposition::Retry,
        _ => RetryDisposition::DoNotRetry,
    }
}

/// Honor a server retry hint only when it is a small, positive delta-seconds
/// value. HTTP-date parsing would make tests clock-dependent and is not needed
/// for the bounded installer retry contract.
pub fn bounded_retry_after(value: Option<&str>) -> Option<Duration> {
    let seconds = value?.trim().parse::<u64>().ok()?;
    (seconds > 0 && seconds <= MAX_RETRY_AFTER_SECS).then(|| Duration::from_secs(seconds))
}

fn validate_https_url(url: &Url) -> Result<(), RedirectPolicyError> {
    if url.scheme() != "https" || url.host_str().is_none() {
        return Err(RedirectPolicyError::InsecureScheme);
    }

    if !url.username().is_empty() || url.password().is_some() {
        return Err(RedirectPolicyError::UserInfo);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::{future, stream};
    use std::collections::VecDeque;
    use std::sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering as AtomicOrdering},
        Arc, Mutex,
    };
    use uuid::Uuid;

    struct FakeTransport {
        responses: Mutex<VecDeque<TransportResponse>>,
        request_count: AtomicUsize,
    }

    impl FakeTransport {
        fn new(responses: impl IntoIterator<Item = TransportResponse>) -> Self {
            Self {
                responses: Mutex::new(responses.into_iter().collect()),
                request_count: AtomicUsize::new(0),
            }
        }

        fn request_count(&self) -> usize {
            self.request_count.load(AtomicOrdering::SeqCst)
        }
    }

    impl HttpTransport for FakeTransport {
        fn get(&self, _url: Url) -> TransportFuture<'_> {
            self.request_count.fetch_add(1, AtomicOrdering::SeqCst);
            let response = self
                .responses
                .lock()
                .unwrap()
                .pop_front()
                .ok_or_else(|| TransportError::non_retryable("unexpected request"));
            Box::pin(async move { response })
        }
    }

    struct PendingTransport;

    impl HttpTransport for PendingTransport {
        fn get(&self, _url: Url) -> TransportFuture<'_> {
            Box::pin(future::pending())
        }
    }

    fn response(status: u16, location: Option<&str>) -> TransportResponse {
        TransportResponse {
            status,
            location: location.map(str::to_owned),
            content_length: None,
            retry_after: None,
            body: Box::pin(stream::empty()),
        }
    }

    fn body_response<I>(status: u16, content_length: Option<u64>, chunks: I) -> TransportResponse
    where
        I: IntoIterator<Item = Result<Bytes, TransportError>>,
        I::IntoIter: Send + 'static,
    {
        TransportResponse {
            status,
            location: None,
            content_length,
            retry_after: None,
            body: Box::pin(stream::iter(chunks)),
        }
    }

    fn fixture_release(expected_bytes: &[u8]) -> ReleaseDescriptor {
        ReleaseDescriptor::new(
            super::super::types::DesktopPlatform::Windows,
            super::super::types::CpuArchitecture::X86_64,
            "1.2.3.4",
            super::super::types::PlatformVersion::parse_windows_msix("1.2.3.4").unwrap(),
            "OpenAI.Codex_1.2.3.4_x64__fixture.Msix",
            verify::sha256_hex(expected_bytes),
            expected_bytes.len() as u64,
            TrustedDownloadEndpoint::WinX64,
            None,
        )
        .unwrap()
    }

    fn assert_no_artifact_files(directory: &Path) {
        assert!(!directory.join("installer.msix.part").exists());
        assert!(!directory.join("installer.msix").exists());
        assert!(!directory.join("installer.dmg.part").exists());
        assert!(!directory.join("installer.dmg").exists());
    }

    fn test_job_directory(root: &tempfile::TempDir) -> JobTempDir {
        JobTempDir::create(root.path(), &Uuid::new_v4().hyphenated().to_string()).unwrap()
    }

    struct SharedCancellation(Arc<AtomicBool>);

    impl Cancellation for SharedCancellation {
        fn is_cancelled(&self) -> bool {
            self.0.load(AtomicOrdering::Acquire)
        }
    }

    #[test]
    fn allows_at_most_five_https_redirects() {
        let mut current = Url::parse("https://mirror.example.test/latest/win-x64").unwrap();

        for followed in 0..MAX_REDIRECTS {
            current = resolve_redirect(&current, "/redirect", followed).unwrap();
        }

        assert_eq!(
            resolve_redirect(&current, "/sixth", MAX_REDIRECTS),
            Err(RedirectPolicyError::TooManyRedirects)
        );
    }

    #[test]
    fn resolves_relative_location_but_rejects_insecure_or_userinfo_targets() {
        let current = Url::parse("https://mirror.example.test/latest/win-x64").unwrap();

        assert_eq!(
            resolve_redirect(&current, "../artifact", 0)
                .unwrap()
                .as_str(),
            "https://mirror.example.test/artifact"
        );
        assert_eq!(
            resolve_redirect(&current, "http://cdn.example.test/package", 0),
            Err(RedirectPolicyError::InsecureScheme)
        );
        assert_eq!(
            resolve_redirect(&current, "https://token@cdn.example.test/package", 0),
            Err(RedirectPolicyError::UserInfo)
        );
    }

    #[test]
    fn diagnostics_never_include_query_fragment_or_userinfo() {
        let url =
            Url::parse("https://secret@cdn.example.test:8443/file?token=hidden#fragment").unwrap();

        assert_eq!(diagnostic_url(&url), "https://cdn.example.test:8443/file");
    }

    #[test]
    fn only_transient_statuses_are_retried() {
        assert_eq!(retry_disposition_for_status(408), RetryDisposition::Retry);
        assert_eq!(retry_disposition_for_status(429), RetryDisposition::Retry);
        assert_eq!(retry_disposition_for_status(503), RetryDisposition::Retry);
        assert_eq!(
            retry_disposition_for_status(404),
            RetryDisposition::DoNotRetry
        );
        assert_eq!(
            retry_disposition_for_status(400),
            RetryDisposition::DoNotRetry
        );
    }

    #[test]
    fn bounds_retry_after_to_keep_the_job_responsive() {
        assert_eq!(bounded_retry_after(Some("2")), Some(Duration::from_secs(2)));
        assert_eq!(bounded_retry_after(Some("0")), None);
        assert_eq!(bounded_retry_after(Some("31")), None);
        assert_eq!(bounded_retry_after(Some("invalid")), None);
    }

    #[tokio::test]
    async fn follows_five_redirects_and_returns_the_terminal_response() {
        let transport = FakeTransport::new([
            response(302, Some("/one")),
            response(302, Some("/two")),
            response(302, Some("/three")),
            response(302, Some("/four")),
            response(302, Some("/five")),
            response(200, None),
        ]);
        let cancellation = super::super::cancellation::NeverCancelled;

        let terminal = get_with_redirects(
            &transport,
            Url::parse("https://mirror.example.test/latest/win-x64").unwrap(),
            &cancellation,
        )
        .await
        .unwrap();

        assert_eq!(terminal.status, 200);
    }

    #[tokio::test]
    async fn rejects_a_sixth_redirect_or_missing_location() {
        let transport = FakeTransport::new([
            response(302, Some("/one")),
            response(302, Some("/two")),
            response(302, Some("/three")),
            response(302, Some("/four")),
            response(302, Some("/five")),
            response(302, Some("/six")),
        ]);
        let cancellation = super::super::cancellation::NeverCancelled;

        assert!(matches!(
            get_with_redirects(
                &transport,
                Url::parse("https://mirror.example.test/latest/win-x64").unwrap(),
                &cancellation,
            )
            .await,
            Err(RedirectRequestError::Redirect(
                RedirectPolicyError::TooManyRedirects
            ))
        ));

        let missing = FakeTransport::new([response(302, None)]);
        assert!(matches!(
            get_with_redirects(
                &missing,
                Url::parse("https://mirror.example.test/latest/win-x64").unwrap(),
                &cancellation,
            )
            .await,
            Err(RedirectRequestError::Redirect(
                RedirectPolicyError::InvalidLocation
            ))
        ));
    }

    #[test]
    fn timeout_transport_errors_keep_the_timeout_code_and_are_retryable() {
        let error = transport_attempt_error(TransportError::timeout("read timed out"));

        assert!(error.retryable);
        assert_eq!(error.error.code(), InstallerErrorCode::DownloadTimeout);
    }

    #[test]
    fn production_transport_is_only_constructed_from_the_clean_builder() {
        let download_client = build_installer_http_client(
            InstallerHttpClientOptions::for_download(None, "fyagent-test"),
        );
        let metadata_client = build_installer_http_client(
            InstallerHttpClientOptions::for_metadata(None, "fyagent-test"),
        );

        assert!(download_client.is_ok());
        assert!(metadata_client.is_ok());
        assert!(
            ReqwestInstallerTransport::new(InstallerHttpClientOptions::for_download(
                None,
                "fyagent-test"
            ))
            .is_ok()
        );
    }

    #[tokio::test]
    async fn downloads_to_a_fixed_local_name_and_emits_bounded_progress() {
        let expected_bytes = b"validated installer bytes";
        let release = fixture_release(expected_bytes);
        let transport = FakeTransport::new([body_response(
            200,
            Some(expected_bytes.len() as u64),
            [
                Ok(Bytes::from_static(b"validated ")),
                Ok(Bytes::from_static(b"installer bytes")),
            ],
        )]);
        let directory = tempfile::tempdir().unwrap();
        let job_directory = test_job_directory(&directory);
        let cancellation = AtomicBool::new(false);
        let updates = Mutex::new(Vec::new());
        let progress = |update| updates.lock().unwrap().push(update);

        let artifact = download_release(
            &transport,
            &release,
            &job_directory,
            &cancellation,
            &progress,
        )
        .await
        .unwrap();

        assert_eq!(artifact.path(), job_directory.path().join("installer.msix"));
        assert_eq!(artifact.size, expected_bytes.len() as u64);
        assert_eq!(std::fs::read(artifact.path()).unwrap(), expected_bytes);
        assert!(!job_directory.path().join("installer.msix.part").exists());
        assert_eq!(transport.request_count(), 1);
        let updates = updates.lock().unwrap();
        assert_eq!(updates.first().unwrap().completed_bytes, 0);
        assert_eq!(
            updates.last().unwrap().completed_bytes,
            expected_bytes.len() as u64
        );
        assert!(updates.iter().all(|update| {
            update.total_bytes == expected_bytes.len() as u64
                && update.completed_bytes <= expected_bytes.len() as u64
        }));
        assert!(updates
            .iter()
            .any(|update| update.phase == ProgressPhase::Verification));
    }

    #[tokio::test]
    async fn revalidation_reopens_the_fixed_job_artifact_and_rejects_a_same_size_replacement() {
        let trusted_bytes = b"trusted";
        let release = fixture_release(trusted_bytes);
        let transport = FakeTransport::new([body_response(
            200,
            Some(trusted_bytes.len() as u64),
            [Ok(Bytes::from_static(trusted_bytes))],
        )]);
        let root = tempfile::tempdir().unwrap();
        let job_directory = test_job_directory(&root);
        let cancellation = AtomicBool::new(false);
        let artifact =
            download_release(&transport, &release, &job_directory, &cancellation, &|_| {})
                .await
                .unwrap();

        std::fs::write(artifact.path(), b"mutated").unwrap();
        let error = artifact
            .revalidate_against(&release)
            .expect_err("a post-download replacement must fail before platform consumption");

        assert_eq!(error.code(), InstallerErrorCode::ChecksumMismatch);
    }

    #[tokio::test]
    async fn retries_transient_stream_and_http_failures_then_restarts_from_zero() {
        let expected_bytes = b"second attempt is complete";
        let release = fixture_release(expected_bytes);
        let mut retryable_http_failure = response(503, None);
        retryable_http_failure.retry_after = Some("1".to_owned());
        let mut transient_stream_failure = body_response(
            200,
            None,
            [Err(TransportError::retryable("connection reset"))],
        );
        transient_stream_failure.retry_after = Some("1".to_owned());
        let transport = FakeTransport::new([
            transient_stream_failure,
            retryable_http_failure,
            body_response(
                200,
                Some(expected_bytes.len() as u64),
                [Ok(Bytes::from_static(b"second attempt is complete"))],
            ),
        ]);
        let directory = tempfile::tempdir().unwrap();
        let job_directory = test_job_directory(&directory);
        let cancellation = AtomicBool::new(false);
        let updates = Mutex::new(Vec::new());
        let progress = |update| updates.lock().unwrap().push(update);

        let artifact = download_release(
            &transport,
            &release,
            &job_directory,
            &cancellation,
            &progress,
        )
        .await
        .unwrap();

        assert_eq!(artifact.size, expected_bytes.len() as u64);
        assert_eq!(transport.request_count(), MAX_DOWNLOAD_ATTEMPTS as usize);
        assert_eq!(
            updates
                .lock()
                .unwrap()
                .iter()
                .filter(|update| update.completed_bytes == 0)
                .map(|update| update.attempt)
                .collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        assert!(!job_directory.path().join("installer.msix.part").exists());
    }

    #[tokio::test]
    async fn rejects_content_length_mismatch_without_retrying() {
        let release = fixture_release(b"expected");
        let transport = FakeTransport::new([body_response(
            200,
            Some(999),
            [Ok(Bytes::from_static(b"expected"))],
        )]);
        let directory = tempfile::tempdir().unwrap();
        let job_directory = test_job_directory(&directory);
        let cancellation = AtomicBool::new(false);
        let progress = |_| {};

        let error = download_release(
            &transport,
            &release,
            &job_directory,
            &cancellation,
            &progress,
        )
        .await
        .unwrap_err();

        assert_eq!(error.code(), InstallerErrorCode::DownloadFailed);
        assert_eq!(
            error.to_dto().details.endpoint_kind.as_deref(),
            Some("win-x64")
        );
        assert_eq!(error.to_dto().details.attempt, Some(1));
        assert_eq!(transport.request_count(), 1);
        assert_no_artifact_files(job_directory.path());
    }

    #[tokio::test]
    async fn detects_checksum_mismatch_without_retrying_and_cleans_final_file() {
        let release = fixture_release(b"good");
        let transport = FakeTransport::new([body_response(
            200,
            Some(4),
            [Ok(Bytes::from_static(b"evil"))],
        )]);
        let directory = tempfile::tempdir().unwrap();
        let job_directory = test_job_directory(&directory);
        let cancellation = AtomicBool::new(false);
        let progress = |_| {};

        let error = download_release(
            &transport,
            &release,
            &job_directory,
            &cancellation,
            &progress,
        )
        .await
        .unwrap_err();

        assert_eq!(error.code(), InstallerErrorCode::ChecksumMismatch);
        assert_eq!(transport.request_count(), 1);
        assert_no_artifact_files(job_directory.path());
    }

    #[tokio::test]
    async fn rejects_truncated_or_oversized_bodies_and_cleans_partial_files() {
        for body in [Bytes::from_static(b"abc"), Bytes::from_static(b"abcdefg")] {
            let release = fixture_release(b"abcdef");
            let transport = FakeTransport::new([body_response(200, None, [Ok(body)])]);
            let directory = tempfile::tempdir().unwrap();
            let job_directory = test_job_directory(&directory);
            let cancellation = AtomicBool::new(false);
            let progress = |_| {};

            let error = download_release(
                &transport,
                &release,
                &job_directory,
                &cancellation,
                &progress,
            )
            .await
            .unwrap_err();

            assert_eq!(error.code(), InstallerErrorCode::DownloadFailed);
            assert_eq!(transport.request_count(), 1);
            assert_no_artifact_files(job_directory.path());
        }
    }

    #[tokio::test]
    async fn cancellation_during_a_stream_removes_the_partial_file_and_never_retries() {
        let release = fixture_release(b"ab");
        let cancelled = Arc::new(AtomicBool::new(false));
        let cancellation = SharedCancellation(Arc::clone(&cancelled));
        let cancellation_for_stream = Arc::clone(&cancelled);
        let body = stream::iter([Ok(Bytes::from_static(b"a")), Ok(Bytes::from_static(b"b"))])
            .enumerate()
            .map(move |(index, chunk)| {
                if index == 1 {
                    cancellation_for_stream.store(true, AtomicOrdering::Release);
                }
                chunk
            });
        let transport = FakeTransport::new([TransportResponse {
            status: 200,
            location: None,
            content_length: Some(2),
            retry_after: None,
            body: Box::pin(body),
        }]);
        let directory = tempfile::tempdir().unwrap();
        let job_directory = test_job_directory(&directory);
        let progress = |_| {};

        let error = download_release(
            &transport,
            &release,
            &job_directory,
            &cancellation,
            &progress,
        )
        .await
        .unwrap_err();

        assert_eq!(error.code(), InstallerErrorCode::DownloadCancelled);
        assert_eq!(transport.request_count(), 1);
        assert_no_artifact_files(job_directory.path());
    }

    #[tokio::test]
    async fn cancellation_aborts_a_pending_request_before_any_file_is_created() {
        let release = fixture_release(b"ab");
        let directory = tempfile::tempdir().unwrap();
        let job_directory = test_job_directory(&directory);
        let cancellation = Arc::new(AtomicBool::new(false));
        let cancellation_for_task = Arc::clone(&cancellation);
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            cancellation_for_task.store(true, AtomicOrdering::Release);
        });
        let progress = |_| {};

        let error = download_release(
            &PendingTransport,
            &release,
            &job_directory,
            cancellation.as_ref(),
            &progress,
        )
        .await
        .unwrap_err();

        assert_eq!(error.code(), InstallerErrorCode::DownloadCancelled);
        assert_no_artifact_files(job_directory.path());
    }
}

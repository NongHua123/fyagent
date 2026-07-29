//! 下载产物的纯校验与磁盘空间预检。
//!
//! 这里不负责下载、临时目录创建或平台包身份校验。它只接受已经由 source
//! 锁定的元数据，并把不可信字节、文件大小和所需卷空间收束为可测试的结果。

use std::{
    collections::HashSet,
    fmt,
    fs::File,
    io::{BufReader, Cursor, Read},
    path::Path,
};

use super::{
    error::{InstallerError, InstallerErrorCode},
    types::normalize_sha256,
};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// 下载与安装临时空间的保守预留倍数。
pub const REQUIRED_FREE_SPACE_MULTIPLIER: u64 = 3;

const HASH_READ_BUFFER_SIZE: usize = 64 * 1024;

/// 下载端点允许的产物扩展名。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactKind {
    Msix,
    Dmg,
}

impl ArtifactKind {
    const fn extension(self) -> &'static str {
        match self {
            Self::Msix => "msix",
            Self::Dmg => "dmg",
        }
    }

    /// 下载器在临时 job 目录中使用的固定完成文件名。
    pub const fn fixed_local_file_name(self) -> &'static str {
        match self {
            Self::Msix => "installer.msix",
            Self::Dmg => "installer.dmg",
        }
    }

    /// 下载器在临时 job 目录中使用的固定未完成文件名。
    pub const fn fixed_part_file_name(self) -> &'static str {
        match self {
            Self::Msix => "installer.msix.part",
            Self::Dmg => "installer.dmg.part",
        }
    }
}

/// 卷标识由平台 adapter 提供，作为不透明值只用于在一次预检内去重。
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct VolumeKey(String);

impl VolumeKey {
    /// 创建一个非空、无 NUL 的不透明卷标识。
    pub fn new(value: impl Into<String>) -> Result<Self, DiskSpaceProbeError> {
        let value = value.into();
        if value.is_empty() || value.contains('\0') {
            return Err(DiskSpaceProbeError::InvalidVolumeKey);
        }

        Ok(Self(value))
    }
}

impl fmt::Debug for VolumeKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("VolumeKey(<opaque>)")
    }
}

/// 磁盘 adapter 自身的失败。其内容刻意不携带 OS 文本或路径。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum DiskSpaceProbeError {
    #[error("disk space probe is unavailable")]
    Unavailable,
    #[error("disk space probe returned an invalid volume key")]
    InvalidVolumeKey,
}

/// 由平台层实现的最小磁盘信息边界。
///
/// 当前核心不实现真实磁盘探测；测试和平台 adapter 可以提供此 trait 的实现。相同
/// `VolumeKey` 只会查询一次可用空间，避免临时目录和目标目录同卷时重复探测。
pub trait DiskSpaceProbe: Send + Sync {
    fn volume_key(&self, path: &Path) -> Result<VolumeKey, DiskSpaceProbeError>;

    fn available_bytes(&self, volume: &VolumeKey) -> Result<u64, DiskSpaceProbeError>;
}

/// 计算内存字节的 SHA-256，小写十六进制输出。
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

/// Compares an in-memory SHA-256 against trusted metadata without applying
/// artifact-size rules. Metadata sources use this for their own raw bytes;
/// package artifacts must still go through the exact-size verifier below.
pub(crate) fn sha256_matches(bytes: &[u8], expected_sha256: &str) -> Result<bool, InstallerError> {
    let expected_digest = parse_sha256(expected_sha256)?;
    let actual_digest: [u8; 32] = Sha256::digest(bytes).into();
    Ok(digest_matches(&actual_digest, &expected_digest))
}

/// 校验内存字节的精确大小和 SHA-256。
pub fn verify_bytes(
    bytes: &[u8],
    expected_size: u64,
    expected_sha256: &str,
) -> Result<(), InstallerError> {
    verify_reader(Cursor::new(bytes), expected_size, expected_sha256)
}

/// 流式校验文件的精确大小和 SHA-256。
///
/// 底层读失败只会返回固定分类，避免将完整本地临时路径带入诊断或 IPC。
pub fn verify_file(
    path: &Path,
    expected_size: u64,
    expected_sha256: &str,
) -> Result<(), InstallerError> {
    let file =
        File::open(path).map_err(|_| download_failed("artifact file could not be opened"))?;
    verify_reader(BufReader::new(file), expected_size, expected_sha256)
}

/// 对任意同步 reader 执行流式大小和 SHA-256 校验。
pub fn verify_reader<R>(
    mut reader: R,
    expected_size: u64,
    expected_sha256: &str,
) -> Result<(), InstallerError>
where
    R: Read,
{
    if expected_size == 0 {
        return Err(metadata_invalid("artifact size metadata is invalid"));
    }

    let expected_digest = parse_sha256(expected_sha256)?;
    let mut actual_size = 0_u64;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; HASH_READ_BUFFER_SIZE];

    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|_| download_failed("artifact stream could not be read"))?;
        if read == 0 {
            break;
        }

        actual_size = actual_size
            .checked_add(read as u64)
            .ok_or_else(|| download_failed("artifact size exceeded supported range"))?;
        if actual_size > expected_size {
            return Err(download_failed(
                "artifact size did not match expected metadata",
            ));
        }

        hasher.update(&buffer[..read]);
    }

    if actual_size != expected_size {
        return Err(download_failed(
            "artifact size did not match expected metadata",
        ));
    }

    let actual_digest: [u8; 32] = hasher.finalize().into();
    if !digest_matches(&actual_digest, &expected_digest) {
        return Err(checksum_mismatch());
    }

    Ok(())
}

/// 返回下载/临时文件/安装 staging 所需的保守可用空间。
///
/// 元数据如果无法安全地乘以三，则 fail closed，由调用方按元数据无效处理。
pub fn required_free_space(expected_size: u64) -> Result<u64, InstallerError> {
    if expected_size == 0 {
        return Err(metadata_invalid("artifact size metadata is invalid"));
    }

    expected_size
        .checked_mul(REQUIRED_FREE_SPACE_MULTIPLIER)
        .ok_or_else(|| metadata_invalid("required free-space calculation overflowed"))
}

/// 在所有参与卷上确认至少有 `expected_size * 3` 可用空间。
///
/// `paths` 至少应包含下载临时目录所在卷；平台 adapter 在目标卷不同的时候追加目标路径。
/// 同一 `VolumeKey` 只检查一次，空路径集合也 fail closed。
pub fn ensure_required_disk_space<I, P>(
    probe: &dyn DiskSpaceProbe,
    paths: I,
    expected_size: u64,
) -> Result<(), InstallerError>
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
{
    let required = required_free_space(expected_size)?;
    let mut seen_volumes = HashSet::new();

    for path in paths {
        let volume = probe
            .volume_key(path.as_ref())
            .map_err(|_| internal_error("disk free space could not be determined"))?;

        if !seen_volumes.insert(volume.clone()) {
            continue;
        }

        let available = probe
            .available_bytes(&volume)
            .map_err(|_| internal_error("disk free space could not be determined"))?;
        if available < required {
            return Err(insufficient_disk_space());
        }
    }

    if seen_volumes.is_empty() {
        return Err(internal_error(
            "no volumes were supplied for disk preflight",
        ));
    }

    Ok(())
}

/// 仅接受不能形成路径、且符合目标端点类型的 artifact 文件名。
///
/// 调用方仍必须把下载写到固定本地文件名；此函数只验证镜像元数据中的精确名称，
/// 从不把该名称拼接进本地路径。
pub fn validate_artifact_file_name(
    artifact_file_name: &str,
    kind: ArtifactKind,
) -> Result<(), InstallerError> {
    if is_safe_artifact_file_name(artifact_file_name, kind) {
        Ok(())
    } else {
        Err(metadata_invalid("artifact filename metadata is invalid"))
    }
}

/// `validate_artifact_file_name` 的布尔形式，便于严格 parser 的字段过滤。
pub fn is_safe_artifact_file_name(artifact_file_name: &str, kind: ArtifactKind) -> bool {
    if artifact_file_name.is_empty()
        || artifact_file_name.trim() != artifact_file_name
        || artifact_file_name.contains(['/', '\\', '\0'])
        || artifact_file_name.contains("..")
        || artifact_file_name.chars().any(char::is_control)
    {
        return false;
    }

    let Some((stem, extension)) = artifact_file_name.rsplit_once('.') else {
        return false;
    };
    !stem.is_empty() && extension.eq_ignore_ascii_case(kind.extension())
}

fn parse_sha256(value: &str) -> Result<[u8; 32], InstallerError> {
    let normalized = normalize_sha256(value)
        .map_err(|_| metadata_invalid("expected SHA-256 metadata is invalid"))?;
    let mut digest = [0_u8; 32];

    for (index, chunk) in normalized.as_bytes().chunks_exact(2).enumerate() {
        digest[index] = hex_value(chunk[0]) << 4 | hex_value(chunk[1]);
    }

    Ok(digest)
}

fn hex_value(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        b'A'..=b'F' => byte - b'A' + 10,
        _ => unreachable!("normalize_sha256 validates every input byte"),
    }
}

/// 固定长度 digest 的无早退比较，避免把实现差异扩散到各个调用方。
fn digest_matches(actual: &[u8; 32], expected: &[u8; 32]) -> bool {
    actual
        .iter()
        .zip(expected)
        .fold(0_u8, |difference, (actual, expected)| {
            difference | (actual ^ expected)
        })
        == 0
}

fn metadata_invalid(message: &'static str) -> InstallerError {
    InstallerError::new(InstallerErrorCode::ReleaseMetadataInvalid).with_diagnostic_message(message)
}

fn download_failed(message: &'static str) -> InstallerError {
    InstallerError::new(InstallerErrorCode::DownloadFailed).with_diagnostic_message(message)
}

fn checksum_mismatch() -> InstallerError {
    InstallerError::new(InstallerErrorCode::ChecksumMismatch)
        .with_diagnostic_message("artifact checksum did not match expected metadata")
}

fn insufficient_disk_space() -> InstallerError {
    InstallerError::new(InstallerErrorCode::InsufficientDiskSpace)
        .with_diagnostic_message("a required volume has insufficient free space")
}

fn internal_error(message: &'static str) -> InstallerError {
    InstallerError::new(InstallerErrorCode::InternalError).with_diagnostic_message(message)
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        path::{Path, PathBuf},
        sync::Mutex,
    };

    use tempfile::tempdir;

    use crate::codex_desktop::{
        error::{InstallerError, InstallerErrorCode},
        types::normalize_sha256,
    };

    use super::*;

    const HELLO_SHA256: &str = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";

    fn assert_error_code<T>(result: Result<T, InstallerError>, expected: InstallerErrorCode) {
        let error = match result {
            Ok(_) => panic!("expected installer error {expected:?}"),
            Err(error) => error,
        };

        assert_eq!(error.code(), expected);
    }

    #[test]
    fn normalizes_only_valid_sha256_hex() {
        assert_eq!(
            normalize_sha256(&HELLO_SHA256.to_ascii_uppercase()).unwrap(),
            HELLO_SHA256
        );

        for invalid in [
            "",
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b982",
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b98240",
            "gcf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824",
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b98 4",
        ] {
            assert_error_code(
                normalize_sha256(invalid),
                InstallerErrorCode::ReleaseMetadataInvalid,
            );
        }

        assert_error_code(
            normalize_sha256(&format!(" {HELLO_SHA256}")),
            InstallerErrorCode::ReleaseMetadataInvalid,
        );
        assert_error_code(
            normalize_sha256(&format!("{HELLO_SHA256} ")),
            InstallerErrorCode::ReleaseMetadataInvalid,
        );
    }

    #[test]
    fn verifies_small_bytes_and_rejects_checksum_or_size_mismatch() {
        assert_eq!(sha256_hex(b"hello"), HELLO_SHA256);
        assert!(sha256_matches(b"hello", HELLO_SHA256).unwrap());
        assert!(!sha256_matches(b"goodbye", HELLO_SHA256).unwrap());
        assert!(verify_bytes(b"hello", 5, HELLO_SHA256).is_ok());
        assert_error_code(
            verify_bytes(b"hello", 5, &"0".repeat(64)),
            InstallerErrorCode::ChecksumMismatch,
        );
        assert_error_code(
            verify_bytes(b"hello", 4, HELLO_SHA256),
            InstallerErrorCode::DownloadFailed,
        );
        assert_error_code(
            verify_bytes(b"hello", 0, HELLO_SHA256),
            InstallerErrorCode::ReleaseMetadataInvalid,
        );
    }

    #[test]
    fn verifies_files_without_exposing_their_paths() {
        let directory = tempdir().unwrap();
        let package_path = directory.path().join("installer.msix.part");
        std::fs::write(&package_path, b"hello").unwrap();

        assert!(verify_file(&package_path, 5, HELLO_SHA256).is_ok());
        assert_error_code(
            verify_file(&package_path, 6, HELLO_SHA256),
            InstallerErrorCode::DownloadFailed,
        );
        let missing_path = directory.path().join("missing.msix");
        let error = verify_file(&missing_path, 5, HELLO_SHA256).unwrap_err();
        assert_eq!(error.code(), InstallerErrorCode::DownloadFailed);
        assert!(!error
            .to_dto()
            .details
            .redacted_message
            .unwrap_or_default()
            .contains(missing_path.to_string_lossy().as_ref()));
    }

    #[test]
    fn required_free_space_uses_checked_three_times_multiplier() {
        assert_eq!(required_free_space(12).unwrap(), 36);
        assert_error_code(
            required_free_space(0),
            InstallerErrorCode::ReleaseMetadataInvalid,
        );
        assert_error_code(
            required_free_space(u64::MAX),
            InstallerErrorCode::ReleaseMetadataInvalid,
        );
    }

    #[test]
    fn checks_a_shared_volume_once() {
        let probe = FakeDiskSpaceProbe::new(
            [("temp", "volume-a"), ("target", "volume-a")],
            [("volume-a", 36)],
        );

        assert!(
            ensure_required_disk_space(&probe, [Path::new("temp"), Path::new("target")], 12)
                .is_ok()
        );
        assert_eq!(probe.calls_for("volume-a"), 1);
    }

    #[test]
    fn requires_sufficient_space_on_every_distinct_volume() {
        let probe = FakeDiskSpaceProbe::new(
            [("temp", "volume-a"), ("target", "volume-b")],
            [("volume-a", 36), ("volume-b", 35)],
        );

        assert_error_code(
            ensure_required_disk_space(&probe, [Path::new("temp"), Path::new("target")], 12),
            InstallerErrorCode::InsufficientDiskSpace,
        );
        assert_eq!(probe.calls_for("volume-a"), 1);
        assert_eq!(probe.calls_for("volume-b"), 1);
    }

    #[test]
    fn rejects_unsafe_or_wrong_extension_artifact_names() {
        assert_eq!(ArtifactKind::Msix.fixed_local_file_name(), "installer.msix");
        assert_eq!(
            ArtifactKind::Dmg.fixed_part_file_name(),
            "installer.dmg.part"
        );
        assert!(validate_artifact_file_name("Codex-Setup.MSIX", ArtifactKind::Msix).is_ok());
        assert!(validate_artifact_file_name("ChatGPT.dmg", ArtifactKind::Dmg).is_ok());

        for artifact_file_name in [
            "",
            "../evil.msix",
            "folder/evil.msix",
            "folder\\evil.msix",
            "evil\0.msix",
            "double..dot.msix",
            " padded.msix",
            "artifact.dmg",
        ] {
            assert_error_code(
                validate_artifact_file_name(artifact_file_name, ArtifactKind::Msix),
                InstallerErrorCode::ReleaseMetadataInvalid,
            );
        }
    }

    struct FakeDiskSpaceProbe {
        path_volumes: HashMap<PathBuf, VolumeKey>,
        available: HashMap<VolumeKey, u64>,
        calls: Mutex<HashMap<VolumeKey, usize>>,
    }

    impl FakeDiskSpaceProbe {
        fn new<const PATH_COUNT: usize, const VOLUME_COUNT: usize>(
            path_volumes: [(&str, &str); PATH_COUNT],
            available: [(&str, u64); VOLUME_COUNT],
        ) -> Self {
            Self {
                path_volumes: path_volumes
                    .into_iter()
                    .map(|(path, volume)| {
                        (
                            PathBuf::from(path),
                            VolumeKey::new(volume).expect("valid fake volume"),
                        )
                    })
                    .collect(),
                available: available
                    .into_iter()
                    .map(|(volume, bytes)| {
                        (VolumeKey::new(volume).expect("valid fake volume"), bytes)
                    })
                    .collect(),
                calls: Mutex::new(HashMap::new()),
            }
        }

        fn calls_for(&self, volume: &str) -> usize {
            self.calls
                .lock()
                .unwrap()
                .get(&VolumeKey::new(volume).unwrap())
                .copied()
                .unwrap_or_default()
        }
    }

    impl DiskSpaceProbe for FakeDiskSpaceProbe {
        fn volume_key(&self, path: &Path) -> Result<VolumeKey, DiskSpaceProbeError> {
            self.path_volumes
                .get(path)
                .cloned()
                .ok_or(DiskSpaceProbeError::Unavailable)
        }

        fn available_bytes(&self, volume: &VolumeKey) -> Result<u64, DiskSpaceProbeError> {
            *self
                .calls
                .lock()
                .unwrap()
                .entry(volume.clone())
                .or_default() += 1;
            self.available
                .get(volume)
                .copied()
                .ok_or(DiskSpaceProbeError::Unavailable)
        }
    }

    #[test]
    fn disk_probe_errors_and_empty_volume_lists_fail_closed() {
        let probe = FakeDiskSpaceProbe::new([], []);

        assert_error_code(
            ensure_required_disk_space(&probe, std::iter::empty::<&Path>(), 12),
            InstallerErrorCode::InternalError,
        );
        assert_error_code(
            ensure_required_disk_space(&probe, [Path::new("unknown")], 12),
            InstallerErrorCode::InternalError,
        );
    }
}

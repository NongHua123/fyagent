//! Capability for a single, job-owned installer temporary directory.
//!
//! The downloader receives this capability instead of a caller-provided path.
//! It can therefore only use fixed artifact names under one newly-created UUID
//! child of the canonical installer temp root.

use std::{
    fmt, fs,
    io::ErrorKind,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use uuid::Uuid;

use super::{
    error::{InstallerError, InstallerErrorCode},
    verify::ArtifactKind,
};

const TEMP_ROOT_DIRECTORY_NAME: &str = "fyagent-codex-installer";
const STALE_JOB_DIRECTORY_AGE: Duration = Duration::from_secs(24 * 60 * 60);
const ALL_USERS_JOB_FILE_NAME: &str = "all-users-job.json";

pub(crate) struct JobTempDir {
    root: PathBuf,
    path: PathBuf,
}

impl fmt::Debug for JobTempDir {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("JobTempDir(<redacted>)")
    }
}

impl JobTempDir {
    pub(crate) fn system_root() -> PathBuf {
        std::env::temp_dir().join(TEMP_ROOT_DIRECTORY_NAME)
    }

    /// Removes only job directories that are older than the V1 retention
    /// window. This is deliberately a best-effort startup operation: an
    /// absent root is normal, and suspicious children are retained rather
    /// than recursively removed.
    pub(crate) fn cleanup_stale_system_root() -> Result<usize, InstallerError> {
        Self::cleanup_stale_under(
            &Self::system_root(),
            STALE_JOB_DIRECTORY_AGE,
            SystemTime::now(),
        )
    }

    fn cleanup_stale_under(
        root: &Path,
        minimum_age: Duration,
        now: SystemTime,
    ) -> Result<usize, InstallerError> {
        match fs::symlink_metadata(root) {
            Ok(_) => {}
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(0),
            Err(_) => {
                return Err(temp_error(
                    "installer temporary root could not be inspected for stale cleanup",
                ))
            }
        }

        let canonical_root = canonicalize_directory(root)?;
        let entries = fs::read_dir(&canonical_root).map_err(|_| {
            temp_error("installer temporary root could not be enumerated for stale cleanup")
        })?;
        let mut removed = 0;

        for entry in entries.flatten() {
            let file_name = entry.file_name();
            let Some(job_id) = file_name.to_str() else {
                continue;
            };
            if !is_canonical_job_id(job_id) {
                continue;
            }

            let candidate = entry.path();
            let metadata = match fs::symlink_metadata(&candidate) {
                Ok(metadata) => metadata,
                Err(_) => continue,
            };
            if is_link_or_reparse_point(&candidate).unwrap_or(true) || !metadata.is_dir() {
                continue;
            }
            let modified = match metadata.modified() {
                Ok(modified) => modified,
                Err(_) => continue,
            };
            if !is_stale(modified, now, minimum_age) {
                continue;
            }

            let canonical_path = match canonicalize_directory(&candidate) {
                Ok(path) => path,
                Err(_) => continue,
            };
            if canonical_path.parent() != Some(canonical_root.as_path()) {
                continue;
            }

            let directory = Self {
                root: canonical_root.clone(),
                path: canonical_path,
            };
            if directory.cleanup().is_ok() {
                removed += 1;
            }
        }

        Ok(removed)
    }

    /// Create exactly one canonical UUID direct child. Existing children are
    /// rejected instead of being opened or reused, which prevents a prepared
    /// symlink/reparse point from becoming a download destination.
    pub(crate) fn create(root: &Path, job_id: &str) -> Result<Self, InstallerError> {
        let canonical_job_id = canonical_job_id(job_id)?;
        ensure_root_directory(root)?;
        let canonical_root = canonicalize_directory(root)?;
        let candidate = canonical_root.join(&canonical_job_id);

        match fs::create_dir(&candidate) {
            Ok(()) => {}
            Err(error) if error.kind() == ErrorKind::AlreadyExists => {
                return Err(temp_error(
                    "installer job temporary directory already exists",
                ));
            }
            Err(_) => {
                return Err(temp_error(
                    "installer job temporary directory could not be created",
                ))
            }
        }

        let canonical_path = canonicalize_directory(&candidate)?;
        if canonical_path.parent() != Some(canonical_root.as_path()) {
            let _ = fs::remove_dir(&candidate);
            return Err(temp_error(
                "installer job temporary directory escaped its canonical root",
            ));
        }

        Ok(Self {
            root: canonical_root,
            path: canonical_path,
        })
    }

    /// Re-opens one existing canonical job directory for the Windows-only
    /// elevated provisioning child. It deliberately accepts only the same
    /// UUID direct-child layout created by [`Self::create`], never an
    /// arbitrary directory supplied on the command line.
    pub(crate) fn open_existing(root: &Path, job_id: &str) -> Result<Self, InstallerError> {
        let canonical_job_id = canonical_job_id(job_id)?;
        let canonical_root = canonicalize_directory(root)?;
        let candidate = canonical_root.join(canonical_job_id);
        let canonical_path = canonicalize_directory(&candidate)?;
        if canonical_path.parent() != Some(canonical_root.as_path()) {
            return Err(temp_error(
                "installer job temporary directory escaped its canonical root",
            ));
        }

        Ok(Self {
            root: canonical_root,
            path: canonical_path,
        })
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn part_path(&self, kind: ArtifactKind) -> PathBuf {
        self.path.join(kind.fixed_part_file_name())
    }

    pub(crate) fn final_path(&self, kind: ArtifactKind) -> PathBuf {
        self.path.join(kind.fixed_local_file_name())
    }

    /// Fixed child-process control file holding a nonce-bound all-users job.
    /// This is intentionally not a caller-provided filename.
    pub(crate) fn all_users_job_path(&self) -> PathBuf {
        self.path.join(ALL_USERS_JOB_FILE_NAME)
    }

    /// Check a fixed artifact path before a filesystem operation. This catches
    /// any unexpected directory replacement and ensures no caller can use the
    /// capability to reach outside its one direct child.
    pub(crate) fn validate_artifact_path(&self, path: &Path) -> Result<(), InstallerError> {
        let parent = path
            .parent()
            .ok_or_else(|| temp_error("artifact path has no parent"))?;
        if parent != self.path {
            return Err(temp_error(
                "artifact path is outside its job temporary directory",
            ));
        }

        let file_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| temp_error("artifact path has no safe file name"))?;
        if !matches!(
            file_name,
            "installer.msix" | "installer.msix.part" | "installer.dmg" | "installer.dmg.part"
        ) {
            return Err(temp_error(
                "artifact path is not a fixed installer file name",
            ));
        }

        let current_root = canonicalize_directory(&self.root)?;
        let current_job_directory = canonicalize_directory(&self.path)?;
        if current_root != self.root
            || current_job_directory != self.path
            || current_job_directory.parent() != Some(self.root.as_path())
        {
            return Err(temp_error(
                "installer temporary directory is no longer a trusted directory",
            ));
        }
        Ok(())
    }

    pub(crate) fn validate_existing_artifact(&self, path: &Path) -> Result<(), InstallerError> {
        self.validate_artifact_path(path)?;
        if is_link_or_reparse_point(path)? {
            return Err(temp_error(
                "installer artifact must not be a link or reparse point",
            ));
        }
        let metadata = fs::symlink_metadata(path)
            .map_err(|_| temp_error("installer artifact could not be inspected"))?;
        if !metadata.is_file() {
            return Err(temp_error("installer artifact must be a regular file"));
        }
        Ok(())
    }

    /// Validates a fixed all-users control path before a filesystem operation.
    /// The elevated child uses this instead of trusting its `job-file`
    /// argument directly.
    pub(crate) fn validate_all_users_control_path(
        &self,
        path: &Path,
    ) -> Result<(), InstallerError> {
        let parent = path
            .parent()
            .ok_or_else(|| temp_error("all-users control path has no parent"))?;
        if parent != self.path {
            return Err(temp_error(
                "all-users control path is outside its job temporary directory",
            ));
        }

        let file_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| temp_error("all-users control path has no safe file name"))?;
        if file_name != ALL_USERS_JOB_FILE_NAME {
            return Err(temp_error(
                "all-users control path is not a fixed file name",
            ));
        }

        self.validate_job_directory()
    }

    /// Checks that the all-users job file is an ordinary file inside the
    /// canonical job directory before the unelevated parent starts `runas`.
    /// The elevated child independently binds its JSON read to a native
    /// no-follow handle and never relies on this pre-elevation path check.
    pub(crate) fn validate_existing_all_users_control(
        &self,
        path: &Path,
    ) -> Result<(), InstallerError> {
        self.validate_all_users_control_path(path)?;
        if is_link_or_reparse_point(path)? {
            return Err(temp_error(
                "all-users control file must not be a link or reparse point",
            ));
        }
        let metadata = fs::symlink_metadata(path)
            .map_err(|_| temp_error("all-users control file could not be inspected"))?;
        if !metadata.is_file() {
            return Err(temp_error("all-users control file must be a regular file"));
        }
        Ok(())
    }

    /// Safely removes only the files this capability can have created and then
    /// removes the now-empty job directory. It never walks a directory tree:
    /// unknown entries, links, reparse points, and directories make cleanup
    /// fail closed instead of being recursively removed.
    pub(crate) fn cleanup(&self) -> Result<(), InstallerError> {
        self.validate_job_directory()?;

        for kind in [ArtifactKind::Msix, ArtifactKind::Dmg] {
            for path in [self.part_path(kind), self.final_path(kind)] {
                self.remove_known_artifact(&path)?;
            }
        }

        let all_users_job = self.all_users_job_path();
        self.remove_known_all_users_control(&all_users_job)?;

        fs::remove_dir(&self.path)
            .map_err(|_| temp_error("installer job temporary directory could not be removed"))
    }

    fn validate_job_directory(&self) -> Result<(), InstallerError> {
        let current_root = canonicalize_directory(&self.root)?;
        let current_job_directory = canonicalize_directory(&self.path)?;
        if current_root != self.root
            || current_job_directory != self.path
            || current_job_directory.parent() != Some(self.root.as_path())
        {
            return Err(temp_error(
                "installer temporary directory is no longer a trusted directory",
            ));
        }
        Ok(())
    }

    fn remove_known_artifact(&self, path: &Path) -> Result<(), InstallerError> {
        self.validate_artifact_path(path)?;

        let metadata = match fs::symlink_metadata(path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
            Err(_) => {
                return Err(temp_error(
                    "installer artifact could not be inspected for cleanup",
                ))
            }
        };

        if is_link_or_reparse_point(path)? || !metadata.is_file() {
            return Err(temp_error(
                "installer cleanup refused a non-regular artifact entry",
            ));
        }

        fs::remove_file(path)
            .map_err(|_| temp_error("installer artifact could not be removed during cleanup"))
    }

    fn remove_known_all_users_control(&self, path: &Path) -> Result<(), InstallerError> {
        self.validate_all_users_control_path(path)?;

        let metadata = match fs::symlink_metadata(path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
            Err(_) => {
                return Err(temp_error(
                    "all-users control file could not be inspected for cleanup",
                ))
            }
        };

        if is_link_or_reparse_point(path)? || !metadata.is_file() {
            return Err(temp_error(
                "all-users cleanup refused a non-regular control entry",
            ));
        }

        fs::remove_file(path)
            .map_err(|_| temp_error("all-users control file could not be removed during cleanup"))
    }
}

fn canonical_job_id(value: &str) -> Result<String, InstallerError> {
    if !is_canonical_job_id(value) {
        return Err(temp_error("installer job ID is not a canonical UUID"));
    }
    Ok(value.to_owned())
}

fn is_canonical_job_id(value: &str) -> bool {
    Uuid::parse_str(value)
        .map(|parsed| parsed.hyphenated().to_string() == value)
        .unwrap_or(false)
}

fn is_stale(modified: SystemTime, now: SystemTime, minimum_age: Duration) -> bool {
    now.duration_since(modified)
        .map(|age| age >= minimum_age)
        .unwrap_or(false)
}

fn ensure_root_directory(root: &Path) -> Result<(), InstallerError> {
    match fs::symlink_metadata(root) {
        Ok(_) => {}
        Err(error) if error.kind() == ErrorKind::NotFound => {
            fs::create_dir_all(root)
                .map_err(|_| temp_error("installer temporary root could not be created"))?;
        }
        Err(_) => {
            return Err(temp_error(
                "installer temporary root could not be inspected",
            ))
        }
    }
    canonicalize_directory(root).map(|_| ())
}

fn canonicalize_directory(path: &Path) -> Result<PathBuf, InstallerError> {
    if is_link_or_reparse_point(path)? {
        return Err(temp_error(
            "installer temporary path must not be a link or reparse point",
        ));
    }
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| temp_error("installer temporary path could not be inspected"))?;
    if !metadata.is_dir() {
        return Err(temp_error("installer temporary path must be a directory"));
    }
    fs::canonicalize(path)
        .map_err(|_| temp_error("installer temporary path could not be canonicalized"))
}

#[cfg(windows)]
fn is_link_or_reparse_point(path: &Path) -> Result<bool, InstallerError> {
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| temp_error("installer temporary path could not be inspected"))?;
    Ok(metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0)
}

#[cfg(not(windows))]
fn is_link_or_reparse_point(path: &Path) -> Result<bool, InstallerError> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| temp_error("installer temporary path could not be inspected"))?;
    Ok(metadata.file_type().is_symlink())
}

fn temp_error(message: &str) -> InstallerError {
    InstallerError::new(InstallerErrorCode::InternalError).with_diagnostic_message(message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_only_a_canonical_uuid_direct_child() {
        let root = tempfile::tempdir().unwrap();
        let job_id = Uuid::new_v4().hyphenated().to_string();

        let job_directory = JobTempDir::create(root.path(), &job_id).unwrap();

        assert_eq!(
            job_directory.path().parent(),
            Some(job_directory.root.as_path())
        );
        assert_eq!(
            job_directory.part_path(ArtifactKind::Msix).file_name(),
            Some(std::ffi::OsStr::new("installer.msix.part"))
        );
        assert!(JobTempDir::create(root.path(), &job_id).is_err());
        assert!(JobTempDir::create(root.path(), "not-a-uuid").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn rejects_a_symlink_root_before_creating_a_job_child() {
        use std::os::unix::fs::symlink;

        let container = tempfile::tempdir().unwrap();
        let real_root = container.path().join("real-root");
        fs::create_dir(&real_root).unwrap();
        let link_root = container.path().join("link-root");
        symlink(&real_root, &link_root).unwrap();

        let error =
            JobTempDir::create(&link_root, &Uuid::new_v4().hyphenated().to_string()).unwrap_err();
        assert_eq!(error.code(), InstallerErrorCode::InternalError);
        assert_eq!(fs::read_dir(&real_root).unwrap().count(), 0);
    }

    #[test]
    fn cleanup_removes_only_fixed_artifacts_and_the_empty_job_directory() {
        let root = tempfile::tempdir().unwrap();
        let job_directory =
            JobTempDir::create(root.path(), &Uuid::new_v4().hyphenated().to_string()).unwrap();
        for kind in [ArtifactKind::Msix, ArtifactKind::Dmg] {
            fs::write(job_directory.part_path(kind), b"partial").unwrap();
            fs::write(job_directory.final_path(kind), b"complete").unwrap();
        }
        let path = job_directory.path().to_path_buf();

        job_directory.cleanup().unwrap();

        assert!(!path.exists());
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
    }

    #[test]
    fn cleanup_fails_closed_when_the_job_directory_contains_an_unknown_entry() {
        let root = tempfile::tempdir().unwrap();
        let job_directory =
            JobTempDir::create(root.path(), &Uuid::new_v4().hyphenated().to_string()).unwrap();
        let unknown_path = job_directory.path().join("unrecognized");
        fs::write(&unknown_path, b"do not recursively remove").unwrap();

        let error = job_directory.cleanup().unwrap_err();

        assert_eq!(error.code(), InstallerErrorCode::InternalError);
        assert!(unknown_path.exists());
    }

    #[test]
    fn stale_cleanup_removes_only_expired_canonical_job_directories() {
        let root = tempfile::tempdir().unwrap();
        let job_directory =
            JobTempDir::create(root.path(), &Uuid::new_v4().hyphenated().to_string()).unwrap();
        fs::write(job_directory.final_path(ArtifactKind::Msix), b"complete").unwrap();
        let path = job_directory.path().to_path_buf();
        let unknown = root.path().join("not-a-job-directory");
        fs::create_dir(&unknown).unwrap();

        let removed = JobTempDir::cleanup_stale_under(
            root.path(),
            STALE_JOB_DIRECTORY_AGE,
            SystemTime::now() + STALE_JOB_DIRECTORY_AGE,
        )
        .unwrap();

        assert_eq!(removed, 1);
        assert!(!path.exists());
        assert!(unknown.exists());
    }

    #[test]
    fn stale_cleanup_keeps_fresh_job_directories_and_future_timestamps() {
        let root = tempfile::tempdir().unwrap();
        let job_directory =
            JobTempDir::create(root.path(), &Uuid::new_v4().hyphenated().to_string()).unwrap();
        let path = job_directory.path().to_path_buf();

        let removed = JobTempDir::cleanup_stale_under(
            root.path(),
            STALE_JOB_DIRECTORY_AGE,
            SystemTime::now(),
        )
        .unwrap();

        assert_eq!(removed, 0);
        assert!(path.exists());
        assert!(!is_stale(
            SystemTime::now() + Duration::from_secs(1),
            SystemTime::now(),
            STALE_JOB_DIRECTORY_AGE,
        ));
    }
}

use std::fmt;

const MAX_DOS_PATH_CHARS: usize = 32_767;

/// The only policy outcomes exposed to WiX. The identifier is intentionally
/// stable: MSI logs can be correlated without disclosing the underlying ACL or
/// SID structure to an end user.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DirectoryPolicyError {
    EmptyOrInvalidPath,
    DeviceOrUncPath,
    NotFixedDrive,
    PersistentAclUnsupported,
    ForbiddenKnownFolder,
    RootOrSystemDirectory,
    ReparsePointDetected {
        component_index: usize,
    },
    FinalPathMismatch,
    OwnerNotTrusted,
    DaclUnreadable {
        win32: Option<u32>,
    },
    UnsupportedAce,
    UntrustedWriteAccess,
    ParentDeleteChildAccess,
    TargetNotDedicated,
    PathChangedDuringCheck,
    InternalFailure {
        stage: &'static str,
        win32: Option<u32>,
    },
}

impl DirectoryPolicyError {
    pub const fn code(&self) -> &'static str {
        match self {
            Self::EmptyOrInvalidPath => "FYDIR001",
            Self::DeviceOrUncPath => "FYDIR002",
            Self::NotFixedDrive => "FYDIR003",
            Self::PersistentAclUnsupported => "FYDIR004",
            Self::ForbiddenKnownFolder => "FYDIR005",
            Self::RootOrSystemDirectory => "FYDIR006",
            Self::ReparsePointDetected { .. } => "FYDIR007",
            Self::FinalPathMismatch => "FYDIR008",
            Self::OwnerNotTrusted => "FYDIR009",
            Self::DaclUnreadable { .. } => "FYDIR010",
            Self::UnsupportedAce => "FYDIR011",
            Self::UntrustedWriteAccess => "FYDIR012",
            Self::ParentDeleteChildAccess => "FYDIR013",
            Self::TargetNotDedicated => "FYDIR014",
            Self::PathChangedDuringCheck => "FYDIR015",
            Self::InternalFailure { .. } => "FYDIR099",
        }
    }

    pub const fn stage(&self) -> &'static str {
        match self {
            Self::EmptyOrInvalidPath | Self::DeviceOrUncPath => "syntax",
            Self::NotFixedDrive | Self::PersistentAclUnsupported => "volume",
            Self::ForbiddenKnownFolder | Self::RootOrSystemDirectory => "known-folder",
            Self::ReparsePointDetected { .. }
            | Self::FinalPathMismatch
            | Self::PathChangedDuringCheck => "path",
            Self::OwnerNotTrusted
            | Self::DaclUnreadable { .. }
            | Self::UnsupportedAce
            | Self::UntrustedWriteAccess
            | Self::ParentDeleteChildAccess => "dacl",
            Self::TargetNotDedicated => "target",
            Self::InternalFailure { stage, .. } => stage,
        }
    }

    pub const fn win32(&self) -> Option<u32> {
        match self {
            Self::DaclUnreadable { win32 } | Self::InternalFailure { win32, .. } => *win32,
            _ => None,
        }
    }
}

impl fmt::Display for DirectoryPolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

/// A normalized absolute DOS path. Parsing is intentionally platform-neutral
/// so malformed MSI input is rejected before any Windows filesystem API is
/// called and can be tested on non-Windows CI.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NormalizedDosPath {
    value: String,
    root: String,
    components: Vec<String>,
}

impl NormalizedDosPath {
    pub fn parse(input: &str) -> Result<Self, DirectoryPolicyError> {
        if input.trim().is_empty()
            || input.contains('\0')
            || input.encode_utf16().count() > MAX_DOS_PATH_CHARS
        {
            return Err(DirectoryPolicyError::EmptyOrInvalidPath);
        }

        let candidate = input.replace('/', "\\");
        if candidate.starts_with("\\\\") || candidate.starts_with("\\??\\") {
            return Err(DirectoryPolicyError::DeviceOrUncPath);
        }

        let bytes = candidate.as_bytes();
        if bytes.len() < 3
            || !bytes[0].is_ascii_alphabetic()
            || bytes[1] != b':'
            || bytes[2] != b'\\'
        {
            return Err(DirectoryPolicyError::EmptyOrInvalidPath);
        }
        if candidate[3..].contains(':') {
            return Err(DirectoryPolicyError::DeviceOrUncPath);
        }

        let drive = (bytes[0] as char).to_ascii_uppercase();
        let mut components = Vec::new();
        for component in candidate[3..].split('\\') {
            match component {
                "" | "." => continue,
                ".." => {
                    if components.pop().is_none() {
                        return Err(DirectoryPolicyError::EmptyOrInvalidPath);
                    }
                }
                _ => {
                    if component.ends_with([' ', '.'])
                        || component.chars().any(|character| {
                            character.is_control()
                                || matches!(character, '<' | '>' | ':' | '"' | '|' | '?' | '*')
                        })
                        || is_reserved_device_component(component)
                    {
                        return Err(DirectoryPolicyError::EmptyOrInvalidPath);
                    }
                    components.push(component.to_owned());
                }
            }
        }

        let root = format!("{drive}:\\");
        let value = if components.is_empty() {
            root.clone()
        } else {
            format!("{root}{}", components.join("\\"))
        };
        Ok(Self {
            value,
            root,
            components,
        })
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }

    pub fn volume_root(&self) -> &str {
        &self.root
    }

    pub fn is_volume_root(&self) -> bool {
        self.components.is_empty()
    }

    pub fn component_count(&self) -> usize {
        self.components.len()
    }

    pub fn component_path(&self, count: usize) -> Option<String> {
        if count > self.components.len() {
            return None;
        }
        if count == 0 {
            return Some(self.root.clone());
        }
        Some(format!(
            "{}{}",
            self.root,
            self.components[..count].join("\\")
        ))
    }

    pub fn same_path(&self, other: &Self) -> bool {
        self.value.eq_ignore_ascii_case(&other.value)
    }
}

fn is_reserved_device_component(component: &str) -> bool {
    let basename = component.split('.').next().unwrap_or_default();
    let upper = basename.to_ascii_uppercase();
    matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || matches!(
            upper.as_bytes(),
            [b'C', b'O', b'M', b'1'..=b'9'] | [b'L', b'P', b'T', b'1'..=b'9']
        )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValidationPhase {
    Ui,
    Execute,
}

impl ValidationPhase {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ui => "ui",
            Self::Execute => "execute",
        }
    }
}

#[derive(Clone, Debug)]
pub struct ValidationContext {
    pub requires_existing_product: bool,
    pub expected_existing_product: bool,
    pub trusted_install_dir: Option<NormalizedDosPath>,
}

impl ValidationContext {
    pub fn first_install() -> Self {
        Self {
            requires_existing_product: false,
            expected_existing_product: false,
            trusted_install_dir: None,
        }
    }

    pub fn existing_product(trusted_install_dir: NormalizedDosPath) -> Self {
        Self {
            requires_existing_product: true,
            expected_existing_product: true,
            trusted_install_dir: Some(trusted_install_dir),
        }
    }

    /// A maintenance route without a HKLM-restored directory must not silently
    /// degrade into a first install at a caller-selected path.
    pub fn existing_product_required() -> Self {
        Self {
            requires_existing_product: true,
            expected_existing_product: false,
            trusted_install_dir: None,
        }
    }

    pub fn permits_non_empty_target(
        &self,
        target: &NormalizedDosPath,
        marker_is_regular_file: bool,
    ) -> bool {
        self.expected_existing_product
            && marker_is_regular_file
            && self
                .trusted_install_dir
                .as_ref()
                .is_some_and(|trusted| trusted.same_path(target))
    }
}

/// A newly created installation directory receives inheritable allow ACEs
/// after the parent itself has been evaluated. Current-parent effective access
/// therefore cannot prove an untrusted future-child grant harmless.
pub const fn future_child_write_is_unsafe(
    applies_to_new_child: bool,
    inheriting_principal_is_safe: bool,
    grants_dangerous_access: bool,
) -> bool {
    applies_to_new_child && !inheriting_principal_is_safe && grants_dangerous_access
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TargetState {
    Missing,
    Empty,
    ExistingProduct,
}

impl TargetState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::Empty => "empty",
            Self::ExistingProduct => "existing-product",
        }
    }
}

#[derive(Clone, Debug)]
pub struct ValidatedDirectory {
    pub normalized_path: NormalizedDosPath,
    pub nearest_existing_ancestor: String,
    pub target_state: TargetState,
}

#[derive(Clone, Debug)]
pub struct ValidationResult {
    pub normalized_path: Option<NormalizedDosPath>,
    pub outcome: Result<ValidatedDirectory, DirectoryPolicyError>,
}

impl ValidationResult {
    pub fn rejected(error: DirectoryPolicyError) -> Self {
        Self {
            normalized_path: None,
            outcome: Err(error),
        }
    }

    pub fn rejected_with_path(path: NormalizedDosPath, error: DirectoryPolicyError) -> Self {
        Self {
            normalized_path: Some(path),
            outcome: Err(error),
        }
    }

    pub fn allowed(directory: ValidatedDirectory) -> Self {
        Self {
            normalized_path: Some(directory.normalized_path.clone()),
            outcome: Ok(directory),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_absolute_dos_paths_without_following_a_filesystem() {
        let path = NormalizedDosPath::parse("d:/Applications/./FyAgent/../FyAgent\\").unwrap();
        assert_eq!(path.as_str(), "D:\\Applications\\FyAgent");
        assert_eq!(path.volume_root(), "D:\\");
        assert_eq!(path.component_path(1).as_deref(), Some("D:\\Applications"));
    }

    #[test]
    fn rejects_ambiguous_or_non_dos_input() {
        for path in [
            "",
            "FyAgent",
            "C:relative",
            "\\\\server\\share\\FyAgent",
            "\\\\?\\C:\\FyAgent",
            "C:\\FyAgent:stream",
            "C:\\..\\FyAgent",
            "C:\\NUL",
            "C:\\FyAgent. ",
        ] {
            assert!(NormalizedDosPath::parse(path).is_err(), "{path}");
        }
    }

    #[test]
    fn keeps_volume_root_distinct_from_a_target_directory() {
        assert!(NormalizedDosPath::parse("C:\\").unwrap().is_volume_root());
        assert!(!NormalizedDosPath::parse("C:\\FyAgent")
            .unwrap()
            .is_volume_root());
    }

    #[test]
    fn inherited_untrusted_write_grants_are_not_masked_by_parent_access() {
        assert!(future_child_write_is_unsafe(true, false, true));
        assert!(!future_child_write_is_unsafe(true, true, true));
        assert!(!future_child_write_is_unsafe(true, false, false));
        assert!(!future_child_write_is_unsafe(false, false, true));
    }

    #[test]
    fn only_a_matching_hklm_backed_existing_context_can_admit_non_empty_content() {
        let target = NormalizedDosPath::parse("D:\\Applications\\FyAgent").unwrap();
        assert!(!ValidationContext::first_install().permits_non_empty_target(&target, true));
        assert!(!ValidationContext::existing_product(target.clone())
            .permits_non_empty_target(&target, false));
        assert!(!ValidationContext::existing_product(
            NormalizedDosPath::parse("D:\\Other\\FyAgent").unwrap(),
        )
        .permits_non_empty_target(&target, true));
        assert!(ValidationContext::existing_product(target.clone())
            .permits_non_empty_target(&target, true));
        assert!(
            !ValidationContext::existing_product_required().permits_non_empty_target(&target, true)
        );
    }

    #[test]
    fn matches_install_directories_case_insensitively_only_after_normalization() {
        let first = NormalizedDosPath::parse("d:\\Applications\\FyAgent").unwrap();
        let second = NormalizedDosPath::parse("D:\\applications\\fyagent\\").unwrap();
        let sibling = NormalizedDosPath::parse("D:\\Applications\\FyAgent2").unwrap();
        assert!(first.same_path(&second));
        assert!(!first.same_path(&sibling));
    }

    #[test]
    fn keeps_the_documented_error_codes_stable() {
        assert_eq!(
            DirectoryPolicyError::UntrustedWriteAccess.code(),
            "FYDIR012"
        );
        assert_eq!(
            DirectoryPolicyError::InternalFailure {
                stage: "ffi",
                win32: None,
            }
            .code(),
            "FYDIR099"
        );
    }
}

use crate::policy::DirectoryPolicyError;

pub(crate) const MSG_LOCAL_FIXED_DRIVE: &str = "无法安装到该位置。请选择本机固定磁盘上的文件夹。";
pub(crate) const MSG_PROTECTED_APPLICATION_FOLDER: &str =
    "无法安装到该位置。请选择 Program Files 或由管理员管理的应用文件夹。";
pub(crate) const MSG_REPARSE_OR_REDIRECT: &str = "该路径包含链接或重定向。请选择其他文件夹。";
pub(crate) const MSG_UNSAFE_PERMISSIONS: &str =
    "该文件夹可能被普通用户修改。请选择受保护的应用文件夹。";
pub(crate) const MSG_NOT_DEDICATED: &str = "请选择一个新的 FyAgent 文件夹，或先清空所选文件夹。";
pub(crate) const MSG_CANNOT_CONFIRM: &str = "无法确认该文件夹是否安全。请选择其他文件夹。";

/// Maps a stable policy error to a deliberately small user-facing vocabulary.
/// Detailed Win32 state remains in the MSI log rather than in the dialog.
pub(crate) fn user_message(error: &DirectoryPolicyError) -> &'static str {
    match error {
        DirectoryPolicyError::DeviceOrUncPath
        | DirectoryPolicyError::NotFixedDrive
        | DirectoryPolicyError::PersistentAclUnsupported => MSG_LOCAL_FIXED_DRIVE,
        DirectoryPolicyError::ForbiddenKnownFolder
        | DirectoryPolicyError::RootOrSystemDirectory => MSG_PROTECTED_APPLICATION_FOLDER,
        DirectoryPolicyError::ReparsePointDetected { .. }
        | DirectoryPolicyError::FinalPathMismatch
        | DirectoryPolicyError::PathChangedDuringCheck => MSG_REPARSE_OR_REDIRECT,
        DirectoryPolicyError::OwnerNotTrusted
        | DirectoryPolicyError::DaclUnreadable { .. }
        | DirectoryPolicyError::UnsupportedAce
        | DirectoryPolicyError::UntrustedWriteAccess
        | DirectoryPolicyError::ParentDeleteChildAccess => MSG_UNSAFE_PERMISSIONS,
        DirectoryPolicyError::TargetNotDedicated => MSG_NOT_DEDICATED,
        DirectoryPolicyError::EmptyOrInvalidPath | DirectoryPolicyError::InternalFailure { .. } => {
            MSG_CANNOT_CONFIRM
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_internal_details_into_a_safe_user_message() {
        assert_eq!(
            user_message(&DirectoryPolicyError::DaclUnreadable { win32: Some(5) }),
            MSG_UNSAFE_PERMISSIONS
        );
        assert_eq!(
            user_message(&DirectoryPolicyError::InternalFailure {
                stage: "known-folder",
                win32: None,
            }),
            MSG_CANNOT_CONFIRM
        );
    }
}

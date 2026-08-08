use std::ptr;

use windows_sys::Win32::{
    Foundation::{
        CloseHandle, GetLastError, ERROR_FILE_NOT_FOUND, ERROR_NO_MORE_FILES, ERROR_PATH_NOT_FOUND,
        HANDLE, INVALID_HANDLE_VALUE,
    },
    Globalization::{CompareStringOrdinal, CSTR_EQUAL},
    Storage::FileSystem::{
        CreateFileW, FileAttributeTagInfo, FindClose, FindFirstFileW, FindNextFileW, GetDriveTypeW,
        GetFileAttributesW, GetFileInformationByHandle, GetFileInformationByHandleEx,
        GetFinalPathNameByHandleW, GetTempPathW, GetVolumeInformationW, BY_HANDLE_FILE_INFORMATION,
        FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_REPARSE_POINT, FILE_ATTRIBUTE_TAG_INFO,
        FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_LIST_DIRECTORY,
        FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, INVALID_FILE_ATTRIBUTES,
        OPEN_EXISTING, READ_CONTROL, VOLUME_NAME_DOS, WIN32_FIND_DATAW,
    },
    System::{
        Com::CoTaskMemFree, SystemServices::FILE_PERSISTENT_ACLS, WindowsProgramming::DRIVE_FIXED,
    },
    UI::Shell::{
        FOLDERID_Desktop, FOLDERID_Documents, FOLDERID_Downloads, FOLDERID_LocalAppData,
        FOLDERID_LocalAppDataLow, FOLDERID_Profile, FOLDERID_ProgramData, FOLDERID_ProgramFiles,
        FOLDERID_RoamingAppData, FOLDERID_System, FOLDERID_SystemX86, FOLDERID_Windows,
        SHGetKnownFolderPath,
    },
};

use crate::{
    policy::{
        DirectoryPolicyError, NormalizedDosPath, TargetState, ValidatedDirectory,
        ValidationContext, ValidationResult,
    },
    security,
};

const PRODUCT_MARKER: &str = "FyAgent.exe";

pub(crate) fn validate_install_directory(
    requested: &str,
    context: &ValidationContext,
) -> ValidationResult {
    let path = match NormalizedDosPath::parse(requested) {
        Ok(path) => path,
        Err(error) => return ValidationResult::rejected(error),
    };
    if path.is_volume_root() {
        return ValidationResult::rejected_with_path(
            path,
            DirectoryPolicyError::RootOrSystemDirectory,
        );
    }
    if let Err(error) = validate_volume(&path) {
        return ValidationResult::rejected_with_path(path, error);
    }
    if let Err(error) = reject_known_folders(&path) {
        return ValidationResult::rejected_with_path(path, error);
    }

    let existing_paths = match existing_components(&path) {
        Ok(paths) => paths,
        Err(error) => return ValidationResult::rejected_with_path(path, error),
    };
    let target_exists = existing_paths.len() == path.component_count() + 1;
    let nearest_existing_index = existing_paths.len().saturating_sub(1);

    let mut opened = Vec::with_capacity(existing_paths.len());
    for (index, component) in existing_paths.iter().enumerate() {
        // A low-privilege principal with DELETE_CHILD on any existing
        // ancestor can delete and replace a validated descendant after this
        // immediate custom action returns. Check the complete chain rather
        // than only the direct parent so the no-follow snapshots cannot be
        // invalidated by a later junction substitution.
        let include_delete_child = true;
        let inspect_inherited_child_aces = !target_exists && index == nearest_existing_index;
        let opened_component = match OpenedDirectory::open(
            component,
            index,
            include_delete_child,
            inspect_inherited_child_aces,
        ) {
            Ok(opened_component) => opened_component,
            Err(error) => return ValidationResult::rejected_with_path(path, error),
        };
        opened.push(opened_component);
    }

    let target_state = if target_exists {
        match directory_is_empty(&path) {
            Ok(true) if !context.requires_existing_product => TargetState::Empty,
            Ok(true) => {
                return ValidationResult::rejected_with_path(
                    path,
                    DirectoryPolicyError::TargetNotDedicated,
                )
            }
            Ok(false) => {
                let marker = product_marker_is_regular_file(&path);
                if context.permits_non_empty_target(&path, marker) {
                    TargetState::ExistingProduct
                } else {
                    return ValidationResult::rejected_with_path(
                        path,
                        DirectoryPolicyError::TargetNotDedicated,
                    );
                }
            }
            Err(error) => return ValidationResult::rejected_with_path(path, error),
        }
    } else if context.requires_existing_product {
        return ValidationResult::rejected_with_path(
            path,
            DirectoryPolicyError::TargetNotDedicated,
        );
    } else {
        TargetState::Missing
    };

    for component in &opened {
        if let Err(error) = component.recheck() {
            return ValidationResult::rejected_with_path(path, error);
        }
    }

    let nearest_existing_ancestor = existing_paths[nearest_existing_index].clone();
    ValidationResult::allowed(ValidatedDirectory {
        normalized_path: path,
        nearest_existing_ancestor,
        target_state,
    })
}

fn validate_volume(path: &NormalizedDosPath) -> Result<(), DirectoryPolicyError> {
    let root = wide_null(path.volume_root());
    if unsafe { GetDriveTypeW(root.as_ptr()) } != DRIVE_FIXED {
        return Err(DirectoryPolicyError::NotFixedDrive);
    }

    let mut filesystem_flags = 0_u32;
    if unsafe {
        GetVolumeInformationW(
            root.as_ptr(),
            ptr::null_mut(),
            0,
            ptr::null_mut(),
            ptr::null_mut(),
            &mut filesystem_flags,
            ptr::null_mut(),
            0,
        )
    } == 0
    {
        return Err(internal_error("volume", Some(last_error())));
    }
    if filesystem_flags & FILE_PERSISTENT_ACLS == 0 {
        return Err(DirectoryPolicyError::PersistentAclUnsupported);
    }
    Ok(())
}

fn reject_known_folders(path: &NormalizedDosPath) -> Result<(), DirectoryPolicyError> {
    let descendants = [
        FOLDERID_Profile,
        FOLDERID_Desktop,
        FOLDERID_Documents,
        FOLDERID_Downloads,
        FOLDERID_LocalAppData,
        FOLDERID_RoamingAppData,
        FOLDERID_LocalAppDataLow,
        FOLDERID_ProgramData,
        FOLDERID_Windows,
        FOLDERID_System,
        FOLDERID_SystemX86,
    ];
    for folder in descendants {
        let root = known_folder_path(&folder)?;
        if is_same_or_child_windows(path, &root) {
            return Err(DirectoryPolicyError::ForbiddenKnownFolder);
        }
    }

    let temp = NormalizedDosPath::parse(&temporary_directory_path()?)
        .map_err(|_| internal_error("temp-folder", None))?;
    if is_same_or_child_windows(path, &temp) {
        return Err(DirectoryPolicyError::ForbiddenKnownFolder);
    }

    let program_files = known_folder_path(&FOLDERID_ProgramFiles)?;
    if same_windows_path(path, &program_files) {
        return Err(DirectoryPolicyError::RootOrSystemDirectory);
    }
    Ok(())
}

fn known_folder_path(
    folder: &windows_sys::core::GUID,
) -> Result<NormalizedDosPath, DirectoryPolicyError> {
    let mut raw = ptr::null_mut();
    let status = unsafe {
        // The current MSI execution token determines user-known folders. The
        // returned buffer is always released with the Shell API's CoTaskMem
        // allocator before this function returns.
        SHGetKnownFolderPath(folder, 0, ptr::null_mut(), &mut raw)
    };
    if status < 0 || raw.is_null() {
        return Err(internal_error("known-folder", Some(status as u32)));
    }
    let value = unsafe { wide_ptr_to_string(raw) };
    unsafe {
        CoTaskMemFree(raw.cast());
    }
    let value = value.map_err(|_| internal_error("known-folder", None))?;
    NormalizedDosPath::parse(&value).map_err(|_| internal_error("known-folder", None))
}

fn temporary_directory_path() -> Result<String, DirectoryPolicyError> {
    let mut capacity = 512_u32;
    loop {
        let mut buffer = vec![0_u16; capacity as usize];
        let length = unsafe { GetTempPathW(capacity, buffer.as_mut_ptr()) };
        if length == 0 {
            return Err(internal_error("temp-folder", Some(last_error())));
        }
        if length < capacity {
            return String::from_utf16(&buffer[..length as usize])
                .map_err(|_| internal_error("temp-folder", None));
        }
        capacity = length
            .checked_add(1)
            .filter(|next| *next <= 32_768)
            .ok_or_else(|| internal_error("temp-folder", None))?;
    }
}

fn existing_components(path: &NormalizedDosPath) -> Result<Vec<String>, DirectoryPolicyError> {
    let mut existing = Vec::new();
    for count in 0..=path.component_count() {
        let component = path
            .component_path(count)
            .ok_or_else(|| internal_error("component", None))?;
        let wide = wide_null(&component);
        let attributes = unsafe { GetFileAttributesW(wide.as_ptr()) };
        if attributes == INVALID_FILE_ATTRIBUTES {
            let error = last_error();
            if matches!(error, ERROR_FILE_NOT_FOUND | ERROR_PATH_NOT_FOUND) {
                break;
            }
            return Err(internal_error("component", Some(error)));
        }
        existing.push(component);
    }
    if existing.is_empty() {
        return Err(internal_error("component", None));
    }
    Ok(existing)
}

fn directory_is_empty(path: &NormalizedDosPath) -> Result<bool, DirectoryPolicyError> {
    let pattern = format!("{}\\*", path.as_str().trim_end_matches('\\'));
    let pattern = wide_null(&pattern);
    let mut data = WIN32_FIND_DATAW::default();
    let search = unsafe { FindFirstFileW(pattern.as_ptr(), &mut data) };
    if search == INVALID_HANDLE_VALUE {
        return Err(internal_error("target-enumeration", Some(last_error())));
    }

    let result = (|| loop {
        let name = file_name(&data).ok_or_else(|| internal_error("target-enumeration", None))?;
        if name != "." && name != ".." {
            return Ok(false);
        }
        if unsafe { FindNextFileW(search, &mut data) } == 0 {
            let error = last_error();
            if error == ERROR_NO_MORE_FILES {
                return Ok(true);
            }
            return Err(internal_error("target-enumeration", Some(error)));
        }
    })();
    unsafe {
        FindClose(search);
    }
    result
}

fn product_marker_is_regular_file(path: &NormalizedDosPath) -> bool {
    let marker = format!("{}\\{PRODUCT_MARKER}", path.as_str().trim_end_matches('\\'));
    let marker = wide_null(&marker);
    let attributes = unsafe { GetFileAttributesW(marker.as_ptr()) };
    attributes != INVALID_FILE_ATTRIBUTES
        && attributes & FILE_ATTRIBUTE_DIRECTORY == 0
        && attributes & FILE_ATTRIBUTE_REPARSE_POINT == 0
}

fn file_name(data: &WIN32_FIND_DATAW) -> Option<String> {
    let length = data.cFileName.iter().position(|unit| *unit == 0)?;
    String::from_utf16(&data.cFileName[..length]).ok()
}

fn same_windows_path(left: &NormalizedDosPath, right: &NormalizedDosPath) -> bool {
    compare_windows_case_insensitive(left.as_str(), right.as_str())
}

fn is_same_or_child_windows(candidate: &NormalizedDosPath, root: &NormalizedDosPath) -> bool {
    if same_windows_path(candidate, root) {
        return true;
    }
    let root = root.as_str().trim_end_matches('\\');
    let candidate_value = candidate.as_str();
    candidate_value
        .as_bytes()
        .get(root.len())
        .is_some_and(|separator| *separator == b'\\')
        && candidate_value.len() > root.len()
        && compare_windows_case_insensitive(&candidate_value[..root.len()], root)
}

fn compare_windows_case_insensitive(left: &str, right: &str) -> bool {
    let left = left.encode_utf16().collect::<Vec<_>>();
    let right = right.encode_utf16().collect::<Vec<_>>();
    unsafe {
        CompareStringOrdinal(
            left.as_ptr(),
            left.len() as i32,
            right.as_ptr(),
            right.len() as i32,
            1,
        ) == CSTR_EQUAL
    }
}

fn final_path_from_handle(handle: HANDLE) -> Result<NormalizedDosPath, DirectoryPolicyError> {
    let mut capacity = 512_u32;
    loop {
        let mut buffer = vec![0_u16; capacity as usize];
        let length = unsafe {
            GetFinalPathNameByHandleW(handle, buffer.as_mut_ptr(), capacity, VOLUME_NAME_DOS)
        };
        if length == 0 {
            return Err(internal_error("final-path", Some(last_error())));
        }
        if length < capacity {
            let value = String::from_utf16(&buffer[..length as usize])
                .map_err(|_| internal_error("final-path", None))?;
            let value = value
                .strip_prefix("\\\\?\\")
                .ok_or(DirectoryPolicyError::FinalPathMismatch)?;
            return NormalizedDosPath::parse(value)
                .map_err(|_| DirectoryPolicyError::FinalPathMismatch);
        }
        capacity = length
            .checked_add(1)
            .filter(|next| *next <= 32_768)
            .ok_or_else(|| internal_error("final-path", None))?;
    }
}

unsafe fn wide_ptr_to_string(raw: *const u16) -> Result<String, ()> {
    let mut length = 0_usize;
    while length < 32_768 {
        // `raw` comes from SHGetKnownFolderPath, which guarantees a
        // NUL-terminated allocation. The bounded scan prevents an unexpected
        // corrupt pointer from driving an unbounded read.
        if unsafe { *raw.add(length) } == 0 {
            let slice = unsafe { std::slice::from_raw_parts(raw, length) };
            return String::from_utf16(slice).map_err(|_| ());
        }
        length += 1;
    }
    Err(())
}

fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn internal_error(stage: &'static str, win32: Option<u32>) -> DirectoryPolicyError {
    DirectoryPolicyError::InternalFailure { stage, win32 }
}

fn last_error() -> u32 {
    unsafe { GetLastError() }
}

struct DirectorySnapshot {
    final_path: NormalizedDosPath,
    volume_serial: u32,
    file_index_high: u32,
    file_index_low: u32,
}

struct OpenedDirectory {
    handle: HANDLE,
    expected_path: NormalizedDosPath,
    component_index: usize,
    include_delete_child: bool,
    inspect_inherited_child_aces: bool,
    snapshot: DirectorySnapshot,
}

impl OpenedDirectory {
    fn open(
        path: &str,
        component_index: usize,
        include_delete_child: bool,
        inspect_inherited_child_aces: bool,
    ) -> Result<Self, DirectoryPolicyError> {
        let expected_path =
            NormalizedDosPath::parse(path).map_err(|_| internal_error("component", None))?;
        let wide = wide_null(path);
        let handle = unsafe {
            // The target is an existing directory. BACKUP_SEMANTICS permits a
            // directory handle; OPEN_REPARSE_POINT lets us inspect and reject
            // a reparse point instead of silently following the leaf.
            CreateFileW(
                wide.as_ptr(),
                // Reading the owner and DACL from this exact no-follow
                // handle requires READ_CONTROL; no write access is requested.
                FILE_LIST_DIRECTORY | READ_CONTROL,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                ptr::null(),
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
                ptr::null_mut(),
            )
        };
        if handle == INVALID_HANDLE_VALUE {
            return Err(internal_error("open-component", Some(last_error())));
        }

        let snapshot = match directory_snapshot(handle, component_index, &expected_path) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                unsafe {
                    CloseHandle(handle);
                }
                return Err(error);
            }
        };
        if let Err(error) = security::validate_directory_security(
            handle,
            include_delete_child,
            inspect_inherited_child_aces,
        ) {
            unsafe {
                CloseHandle(handle);
            }
            return Err(error);
        }
        Ok(Self {
            handle,
            expected_path,
            component_index,
            include_delete_child,
            inspect_inherited_child_aces,
            snapshot,
        })
    }

    fn recheck(&self) -> Result<(), DirectoryPolicyError> {
        let current = directory_snapshot(self.handle, self.component_index, &self.expected_path)?;
        if !same_windows_path(&current.final_path, &self.snapshot.final_path)
            || current.volume_serial != self.snapshot.volume_serial
            || current.file_index_high != self.snapshot.file_index_high
            || current.file_index_low != self.snapshot.file_index_low
        {
            return Err(DirectoryPolicyError::PathChangedDuringCheck);
        }
        security::validate_directory_security(
            self.handle,
            self.include_delete_child,
            self.inspect_inherited_child_aces,
        )
    }
}

impl Drop for OpenedDirectory {
    fn drop(&mut self) {
        if self.handle != INVALID_HANDLE_VALUE {
            unsafe {
                CloseHandle(self.handle);
            }
        }
    }
}

fn directory_snapshot(
    handle: HANDLE,
    component_index: usize,
    expected_path: &NormalizedDosPath,
) -> Result<DirectorySnapshot, DirectoryPolicyError> {
    let mut attributes = FILE_ATTRIBUTE_TAG_INFO::default();
    if unsafe {
        GetFileInformationByHandleEx(
            handle,
            FileAttributeTagInfo,
            (&mut attributes as *mut FILE_ATTRIBUTE_TAG_INFO).cast(),
            std::mem::size_of::<FILE_ATTRIBUTE_TAG_INFO>() as u32,
        )
    } == 0
    {
        return Err(internal_error("attributes", Some(last_error())));
    }
    if attributes.FileAttributes & FILE_ATTRIBUTE_DIRECTORY == 0 {
        return Err(DirectoryPolicyError::TargetNotDedicated);
    }
    if attributes.FileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(DirectoryPolicyError::ReparsePointDetected { component_index });
    }

    let final_path = final_path_from_handle(handle)?;
    if !same_windows_path(&final_path, expected_path) {
        return Err(DirectoryPolicyError::FinalPathMismatch);
    }

    let mut information = BY_HANDLE_FILE_INFORMATION::default();
    if unsafe { GetFileInformationByHandle(handle, &mut information) } == 0 {
        return Err(internal_error("file-id", Some(last_error())));
    }
    Ok(DirectorySnapshot {
        final_path,
        volume_serial: information.dwVolumeSerialNumber,
        file_index_high: information.nFileIndexHigh,
        file_index_low: information.nFileIndexLow,
    })
}

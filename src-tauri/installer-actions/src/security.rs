use std::{mem::offset_of, ptr};

use windows_sys::Win32::{
    Foundation::{GetLastError, LocalFree, ERROR_SUCCESS, HANDLE, HLOCAL, LUID},
    Security::Authorization::{
        AuthzAccessCheck, AuthzFreeContext, AuthzFreeHandle, AuthzFreeResourceManager,
        AuthzInitializeContextFromSid, AuthzInitializeResourceManager, ConvertStringSidToSidW,
        GetSecurityInfo, AUTHZ_ACCESS_REPLY, AUTHZ_ACCESS_REQUEST, AUTHZ_CLIENT_CONTEXT_HANDLE,
        AUTHZ_RESOURCE_MANAGER_HANDLE, AUTHZ_RM_FLAG_NO_AUDIT, SE_FILE_OBJECT,
    },
    Security::{
        AclSizeInformation, CreateWellKnownSid, EqualSid, GetAce, GetAclInformation, GetLengthSid,
        IsValidAcl, IsValidSecurityDescriptor, IsValidSid, IsWellKnownSid, WinAuthenticatedUserSid,
        WinBuiltinAdministratorsSid, WinBuiltinUsersSid, WinCreatorOwnerSid, WinLocalSystemSid,
        WinWorldSid, ACCESS_ALLOWED_ACE, ACE_HEADER, ACL, ACL_SIZE_INFORMATION,
        CONTAINER_INHERIT_ACE, DACL_SECURITY_INFORMATION, INHERIT_ONLY_ACE, OBJECT_INHERIT_ACE,
        OWNER_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, PSID,
    },
    Storage::FileSystem::{
        DELETE, FILE_ADD_FILE, FILE_ADD_SUBDIRECTORY, FILE_DELETE_CHILD, FILE_WRITE_ATTRIBUTES,
        FILE_WRITE_EA, WRITE_DAC, WRITE_OWNER,
    },
};

use crate::policy::{future_child_write_is_unsafe, DirectoryPolicyError};

const ACCESS_ALLOWED_ACE_TYPE: u8 = 0;
const ACCESS_DENIED_ACE_TYPE: u8 = 1;
const GENERIC_ALL_MASK: u32 = 0x1000_0000;
const GENERIC_WRITE_MASK: u32 = 0x4000_0000;
const SECURITY_MAX_SID_SIZE: usize = 68;
const SID_BINARY_HEADER_SIZE: usize = 8;
const TRUSTED_INSTALLER_SID: &str =
    "S-1-5-80-956008885-3418522649-1831038044-1853292631-2271478464";

pub(crate) fn validate_directory_security(
    handle: HANDLE,
    include_delete_child: bool,
    inspect_inherited_child_aces: bool,
) -> Result<(), DirectoryPolicyError> {
    let descriptor = SecurityDescriptor::from_handle(handle)?;
    if descriptor.owner.is_null() || unsafe { IsValidSid(descriptor.owner) } == 0 {
        return Err(DirectoryPolicyError::OwnerNotTrusted);
    }
    if !is_trusted_owner(descriptor.owner)? {
        return Err(DirectoryPolicyError::OwnerNotTrusted);
    }
    if descriptor.dacl.is_null() || unsafe { IsValidAcl(descriptor.dacl) } == 0 {
        return Err(dacl_error(None));
    }

    let requested_access = dangerous_access_mask(include_delete_child);
    let access_checker = AuthzAccessChecker::new()?;
    for principal in [WinWorldSid, WinAuthenticatedUserSid, WinBuiltinUsersSid] {
        let sid = WellKnownSid::new(principal)?;
        if access_checker.grants_any(descriptor.raw, sid.as_psid(), requested_access)? {
            return Err(if include_delete_child {
                DirectoryPolicyError::ParentDeleteChildAccess
            } else {
                DirectoryPolicyError::UntrustedWriteAccess
            });
        }
    }

    let mut information = ACL_SIZE_INFORMATION::default();
    if unsafe {
        GetAclInformation(
            descriptor.dacl,
            (&mut information as *mut ACL_SIZE_INFORMATION).cast(),
            std::mem::size_of::<ACL_SIZE_INFORMATION>() as u32,
            AclSizeInformation,
        )
    } == 0
    {
        return Err(dacl_error(Some(last_error())));
    }

    for index in 0..information.AceCount {
        let mut raw_ace = ptr::null_mut();
        if unsafe { GetAce(descriptor.dacl, index, &mut raw_ace) } == 0 || raw_ace.is_null() {
            return Err(dacl_error(Some(last_error())));
        }
        let header = unsafe {
            // `GetAce` succeeded for an IsValidAcl-validated ACL. Copy the
            // fixed header without assuming any stronger Rust alignment than
            // the Windows byte layout promises.
            ptr::read_unaligned(raw_ace.cast::<ACE_HEADER>())
        };
        match header.AceType {
            ACCESS_DENIED_ACE_TYPE => validate_deny_ace(raw_ace, &header)?,
            ACCESS_ALLOWED_ACE_TYPE => validate_allow_ace(
                raw_ace,
                &header,
                inspect_inherited_child_aces,
                include_delete_child,
            )?,
            _ => return Err(DirectoryPolicyError::UnsupportedAce),
        }
    }
    Ok(())
}

fn validate_deny_ace(
    raw_ace: *mut core::ffi::c_void,
    header: &ACE_HEADER,
) -> Result<(), DirectoryPolicyError> {
    validate_ace_sid(header, raw_ace).map(|_| ())
}

fn validate_allow_ace(
    raw_ace: *mut core::ffi::c_void,
    header: &ACE_HEADER,
    inspect_inherited_child_aces: bool,
    include_delete_child: bool,
) -> Result<(), DirectoryPolicyError> {
    let (mask, sid) = access_allowed_mask_and_sid(raw_ace, header)?;
    let applies_to_current = header.AceFlags & (INHERIT_ONLY_ACE as u8) == 0;
    let applies_to_new_child = inspect_inherited_child_aces
        && header.AceFlags & ((OBJECT_INHERIT_ACE | CONTAINER_INHERIT_ACE) as u8) != 0;
    if !applies_to_current && !applies_to_new_child {
        return Ok(());
    }

    // Program Files commonly has this inherit-only ACE. It does not grant a
    // standard user access to the current ancestor; when MSI creates the child
    // as SYSTEM, CREATOR_OWNER resolves to SYSTEM and WiX immediately applies
    // the final protected DACL. Any nontrusted create/delete capability on the
    // parent is still rejected separately before this exception is reached.
    let inherited_creator_owner =
        header.AceFlags & (INHERIT_ONLY_ACE as u8) != 0 && is_creator_owner(sid)?;
    let trusted_principal = is_trusted_principal(sid)?;
    let dangerous = dangerous_access_mask(include_delete_child);
    let grants_dangerous_access = mask & (GENERIC_ALL_MASK | GENERIC_WRITE_MASK | dangerous) != 0;

    // Authz evaluates the parent's *current* DACL. An inherit-only canonical
    // BU/AU/WD allow ACE can grant no parent access while still granting write
    // access to the new directory MSI creates next. Reject that future-child
    // grant directly; a current-parent Authz result is not relevant evidence.
    if future_child_write_is_unsafe(
        applies_to_new_child,
        inherited_creator_owner || trusted_principal,
        grants_dangerous_access,
    ) {
        return Err(if include_delete_child {
            DirectoryPolicyError::ParentDeleteChildAccess
        } else {
            DirectoryPolicyError::UntrustedWriteAccess
        });
    }

    if inherited_creator_owner || trusted_principal {
        return Ok(());
    }

    if grants_dangerous_access {
        // AuthzAccessCheck already evaluated canonical low-privilege groups
        // with deny/group semantics for the current directory. Future-child
        // grants were rejected above; an arbitrary nontrusted SID cannot be
        // proved harmless from this elevated MSI context, so fail closed.
        if is_canonical_low_privilege_principal(sid)? {
            return Ok(());
        }
        return Err(if include_delete_child {
            DirectoryPolicyError::ParentDeleteChildAccess
        } else {
            DirectoryPolicyError::UntrustedWriteAccess
        });
    }
    Ok(())
}

fn access_allowed_mask_and_sid(
    raw_ace: *mut core::ffi::c_void,
    header: &ACE_HEADER,
) -> Result<(u32, PSID), DirectoryPolicyError> {
    let prefix = offset_of!(ACCESS_ALLOWED_ACE, SidStart);
    if (header.AceSize as usize) < prefix {
        return Err(dacl_error(None));
    }
    let mask = unsafe {
        // `AceSize` covers the SidStart offset, which is after the fixed
        // mask. Read the C-layout field without assuming Rust alignment.
        ptr::read_unaligned(
            (raw_ace as *const u8)
                .add(offset_of!(ACCESS_ALLOWED_ACE, Mask))
                .cast::<u32>(),
        )
    };
    Ok((mask, validate_ace_sid(header, raw_ace)?))
}

fn validate_ace_sid(
    header: &ACE_HEADER,
    raw_ace: *mut core::ffi::c_void,
) -> Result<PSID, DirectoryPolicyError> {
    let prefix = offset_of!(ACCESS_ALLOWED_ACE, SidStart);
    let ace_size = header.AceSize as usize;
    if ace_size < prefix.saturating_add(SID_BINARY_HEADER_SIZE) {
        return Err(dacl_error(None));
    }
    let sid = unsafe {
        // The declared ACE size includes a complete binary SID header, so
        // this pointer stays within the returned ACL allocation.
        (raw_ace as *mut u8).add(prefix).cast()
    };
    let sub_authority_count = unsafe {
        // The SID binary header's second byte is SubAuthorityCount. Its
        // enclosing 8-byte header was bounds-checked above.
        *(sid as *const u8).add(1)
    };
    let sid_length = SID_BINARY_HEADER_SIZE
        .checked_add((sub_authority_count as usize).saturating_mul(std::mem::size_of::<u32>()))
        .ok_or_else(|| dacl_error(None))?;
    if prefix.saturating_add(sid_length) > ace_size || unsafe { IsValidSid(sid) } == 0 {
        return Err(dacl_error(None));
    }
    if unsafe { GetLengthSid(sid) } as usize != sid_length {
        return Err(dacl_error(None));
    }
    Ok(sid)
}

fn dangerous_access_mask(include_delete_child: bool) -> u32 {
    let mut mask = FILE_ADD_FILE
        | FILE_ADD_SUBDIRECTORY
        | FILE_WRITE_EA
        | FILE_WRITE_ATTRIBUTES
        | DELETE
        | WRITE_DAC
        | WRITE_OWNER;
    if include_delete_child {
        mask |= FILE_DELETE_CHILD;
    }
    mask
}

fn is_trusted_owner(sid: PSID) -> Result<bool, DirectoryPolicyError> {
    if unsafe { IsWellKnownSid(sid, WinLocalSystemSid) } != 0
        || unsafe { IsWellKnownSid(sid, WinBuiltinAdministratorsSid) } != 0
    {
        return Ok(true);
    }
    let trusted_installer = StringSid::new(TRUSTED_INSTALLER_SID)?;
    Ok(unsafe { EqualSid(sid, trusted_installer.as_psid()) } != 0)
}

fn is_trusted_principal(sid: PSID) -> Result<bool, DirectoryPolicyError> {
    is_trusted_owner(sid)
}

fn is_creator_owner(sid: PSID) -> Result<bool, DirectoryPolicyError> {
    if unsafe { IsValidSid(sid) } == 0 {
        return Err(dacl_error(None));
    }
    Ok(unsafe { IsWellKnownSid(sid, WinCreatorOwnerSid) } != 0)
}

fn is_canonical_low_privilege_principal(sid: PSID) -> Result<bool, DirectoryPolicyError> {
    if unsafe { IsValidSid(sid) } == 0 {
        return Err(dacl_error(None));
    }
    Ok(unsafe { IsWellKnownSid(sid, WinWorldSid) } != 0
        || unsafe { IsWellKnownSid(sid, WinAuthenticatedUserSid) } != 0
        || unsafe { IsWellKnownSid(sid, WinBuiltinUsersSid) } != 0)
}

fn dacl_error(win32: Option<u32>) -> DirectoryPolicyError {
    DirectoryPolicyError::DaclUnreadable { win32 }
}

fn last_error() -> u32 {
    unsafe { GetLastError() }
}

struct SecurityDescriptor {
    raw: PSECURITY_DESCRIPTOR,
    owner: PSID,
    dacl: *mut ACL,
}

impl SecurityDescriptor {
    fn from_handle(handle: HANDLE) -> Result<Self, DirectoryPolicyError> {
        let mut raw = ptr::null_mut();
        let mut owner = ptr::null_mut();
        let mut dacl = ptr::null_mut();
        let status = unsafe {
            // `handle` is a live directory handle opened with no-follow flags.
            // GetSecurityInfo allocates `raw`, which this type frees with the
            // matching LocalFree allocator in Drop.
            GetSecurityInfo(
                handle,
                SE_FILE_OBJECT,
                OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
                &mut owner,
                ptr::null_mut(),
                &mut dacl,
                ptr::null_mut(),
                &mut raw,
            )
        };
        if status != ERROR_SUCCESS || raw.is_null() {
            if !raw.is_null() {
                unsafe {
                    LocalFree(raw as HLOCAL);
                }
            }
            return Err(dacl_error(Some(status)));
        }
        if unsafe { IsValidSecurityDescriptor(raw) } == 0 {
            unsafe {
                LocalFree(raw as HLOCAL);
            }
            return Err(dacl_error(None));
        }
        Ok(Self { raw, owner, dacl })
    }
}

impl Drop for SecurityDescriptor {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            unsafe {
                LocalFree(self.raw as HLOCAL);
            }
        }
    }
}

struct WellKnownSid {
    bytes: Vec<u8>,
}

impl WellKnownSid {
    fn new(kind: i32) -> Result<Self, DirectoryPolicyError> {
        let mut bytes = vec![0_u8; SECURITY_MAX_SID_SIZE];
        let mut length = bytes.len() as u32;
        if unsafe {
            CreateWellKnownSid(
                kind,
                ptr::null_mut(),
                bytes.as_mut_ptr().cast(),
                &mut length,
            )
        } == 0
        {
            return Err(dacl_error(Some(last_error())));
        }
        bytes.truncate(length as usize);
        Ok(Self { bytes })
    }

    fn as_psid(&self) -> PSID {
        self.bytes.as_ptr().cast_mut().cast()
    }
}

struct StringSid {
    raw: PSID,
}

impl StringSid {
    fn new(value: &str) -> Result<Self, DirectoryPolicyError> {
        let wide = value
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();
        let mut raw = ptr::null_mut();
        if unsafe { ConvertStringSidToSidW(wide.as_ptr(), &mut raw) } == 0 || raw.is_null() {
            return Err(dacl_error(Some(last_error())));
        }
        Ok(Self { raw })
    }

    const fn as_psid(&self) -> PSID {
        self.raw
    }
}

impl Drop for StringSid {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            unsafe {
                LocalFree(self.raw as HLOCAL);
            }
        }
    }
}

struct AuthzAccessChecker {
    resource_manager: AUTHZ_RESOURCE_MANAGER_HANDLE,
}

impl AuthzAccessChecker {
    fn new() -> Result<Self, DirectoryPolicyError> {
        let mut resource_manager = ptr::null_mut();
        if unsafe {
            AuthzInitializeResourceManager(
                AUTHZ_RM_FLAG_NO_AUDIT,
                None,
                None,
                None,
                ptr::null(),
                &mut resource_manager,
            )
        } == 0
            || resource_manager.is_null()
        {
            return Err(dacl_error(Some(last_error())));
        }
        Ok(Self { resource_manager })
    }

    fn grants_any(
        &self,
        descriptor: PSECURITY_DESCRIPTOR,
        sid: PSID,
        desired_access: u32,
    ) -> Result<bool, DirectoryPolicyError> {
        let mut context: AUTHZ_CLIENT_CONTEXT_HANDLE = ptr::null_mut();
        if unsafe {
            AuthzInitializeContextFromSid(
                0,
                sid,
                self.resource_manager,
                ptr::null(),
                LUID::default(),
                ptr::null(),
                &mut context,
            )
        } == 0
            || context.is_null()
        {
            return Err(dacl_error(Some(last_error())));
        }

        let result = (|| {
            let mut granted_access = 0_u32;
            let mut access_error = 0_u32;
            let mut reply = AUTHZ_ACCESS_REPLY {
                ResultListLength: 1,
                GrantedAccessMask: &mut granted_access,
                SaclEvaluationResults: ptr::null_mut(),
                Error: &mut access_error,
            };
            let request = AUTHZ_ACCESS_REQUEST {
                DesiredAccess: desired_access,
                ..Default::default()
            };
            let mut results = ptr::null_mut();
            if unsafe {
                AuthzAccessCheck(
                    0,
                    context,
                    &request,
                    ptr::null_mut(),
                    descriptor,
                    ptr::null(),
                    0,
                    &mut reply,
                    &mut results,
                )
            } == 0
            {
                return Err(dacl_error(Some(last_error())));
            }
            if !results.is_null() {
                unsafe {
                    AuthzFreeHandle(results);
                }
            }
            if access_error != ERROR_SUCCESS {
                return Err(dacl_error(Some(access_error)));
            }
            Ok(granted_access & desired_access != 0)
        })();
        unsafe {
            AuthzFreeContext(context);
        }
        result
    }
}

impl Drop for AuthzAccessChecker {
    fn drop(&mut self) {
        if !self.resource_manager.is_null() {
            unsafe {
                AuthzFreeResourceManager(self.resource_manager);
            }
        }
    }
}

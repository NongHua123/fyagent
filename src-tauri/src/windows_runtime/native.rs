//! Native Windows implementation for the pre-Tauri runtime guard.
//!
//! Release builds use a WiX-owned ProgramData runtime root as the discovery
//! bootstrap. No client ever connects to a pipe derived from a user SID or a
//! predictable mutex name: the protected state descriptor carries a fresh
//! 244-bit pipe endpoint and a separate 256-bit activation capability.

use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver},
        Arc, Mutex, OnceLock,
    },
    thread,
    time::{Duration, Instant},
};

use windows::{
    core::{PCWSTR, PWSTR},
    Win32::{
        Foundation::{
            CloseHandle, GetLastError, LocalFree, SetLastError, ERROR_FILE_NOT_FOUND,
            ERROR_INVALID_PARAMETER, ERROR_LOCK_VIOLATION, ERROR_NOT_ALL_ASSIGNED, ERROR_PIPE_BUSY,
            ERROR_PIPE_CONNECTED, ERROR_SHARING_VIOLATION, FILETIME, HANDLE, HLOCAL,
            INVALID_HANDLE_VALUE, LUID, WIN32_ERROR,
        },
        Security::{
            AdjustTokenPrivileges,
            Authorization::{
                ConvertSecurityDescriptorToStringSecurityDescriptorW, ConvertSidToStringSidW,
                ConvertStringSecurityDescriptorToSecurityDescriptorW, GetSecurityInfo,
            },
            CheckTokenMembership, CreateWellKnownSid,
            Cryptography::{BCryptGenRandom, BCRYPT_USE_SYSTEM_PREFERRED_RNG},
            GetTokenInformation, LookupPrivilegeValueW, RevertToSelf, TokenElevation,
            TokenSessionId, TokenUser, WinBuiltinAdministratorsSid, LUID_AND_ATTRIBUTES,
            PSECURITY_DESCRIPTOR, PSID, SECURITY_ATTRIBUTES, SECURITY_MAX_SID_SIZE,
            SE_PRIVILEGE_ENABLED, SE_RESTORE_NAME, TOKEN_ADJUST_PRIVILEGES, TOKEN_ELEVATION,
            TOKEN_PRIVILEGES, TOKEN_QUERY, TOKEN_USER,
        },
        Storage::FileSystem::{
            CreateFileW, FileAttributeTagInfo, FileDispositionInfo, FlushFileBuffers,
            GetFileInformationByHandleEx, GetFileSizeEx, LockFileEx, ReadFile,
            SetFileInformationByHandle, WriteFile, CREATE_NEW, DELETE, FILE_ATTRIBUTE_DIRECTORY,
            FILE_ATTRIBUTE_NORMAL, FILE_ATTRIBUTE_REPARSE_POINT, FILE_ATTRIBUTE_TAG_INFO,
            FILE_DISPOSITION_INFO, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
            FILE_GENERIC_READ, FILE_GENERIC_WRITE, FILE_READ_ATTRIBUTES, FILE_READ_DATA,
            FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_ALWAYS, OPEN_EXISTING,
            PIPE_ACCESS_DUPLEX, READ_CONTROL, SECURITY_EFFECTIVE_ONLY, SECURITY_IDENTIFICATION,
            SECURITY_SQOS_PRESENT,
        },
        System::{
            Com::CoTaskMemFree,
            Pipes::{
                ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe,
                ImpersonateNamedPipeClient, PeekNamedPipe, WaitNamedPipeW, PIPE_READMODE_MESSAGE,
                PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_MESSAGE, PIPE_WAIT,
            },
            RemoteDesktop::{ProcessIdToSessionId, WTSQueryUserToken},
            Threading::{
                GetCurrentProcess, GetCurrentProcessId, GetCurrentThread, GetProcessTimes,
                OpenProcess, OpenProcessToken, OpenThreadToken, PROCESS_QUERY_LIMITED_INFORMATION,
            },
            IO::OVERLAPPED,
        },
        UI::{
            Shell::{FOLDERID_ProgramData, SHGetKnownFolderPath, KNOWN_FOLDER_FLAG},
            WindowsAndMessaging::{GetShellWindow, GetWindowThreadProcessId},
        },
    },
};

use super::{
    business_instance_key, decide_descriptor_startup, decode_activation_frame,
    decode_handshake_frame, decode_instance_state, encode_activation_auth,
    encode_activation_request, encode_activation_stop, encode_client_hello, encode_instance_state,
    encode_server_proof, evaluate_privilege_gate, formal_windows_build, is_expected_pipe_sddl,
    is_expected_runtime_root_sddl, is_expected_static_object_sddl, verify_activation_auth,
    verify_server_proof, ActivationEnvelope, ActivationFrame, ActivationWireMessage,
    DescriptorLockState, DescriptorReadState, DescriptorStartupDecision, HandshakeMessage,
    InstanceState, InteractiveUserMatch, OwnerLiveness, PrivilegeGateDecision,
    RuntimePrivilegePlatform, RuntimePrivilegeStatus, WindowsStartupDisposition,
    WindowsStartupErrorCode, ACTIVATION_AUTH_FRAME_BYTES, ACTIVATION_FRAME_BYTES,
    HANDSHAKE_CHALLENGE_BYTES, HANDSHAKE_FRAME_BYTES, PIPE_NONCE_BYTES,
    PROTECTED_STATIC_OBJECT_SDDL,
};

const ACTIVATION_QUEUE_LIMIT: usize = 8;
const ACTIVATION_RESPONSE_BYTES: usize = 1;
const ACTIVATION_CONNECT_TIMEOUT: Duration = Duration::from_millis(400);
const ACTIVATION_READ_TIMEOUT: Duration = Duration::from_millis(500);
const ACTIVATION_RELEASE_TIMEOUT: Duration = Duration::from_millis(1200);
const ACTIVATION_POLL_INTERVAL: Duration = Duration::from_millis(10);
const DESCRIPTOR_READ_ATTEMPTS: usize = 25;
const DESCRIPTOR_READ_RETRY: Duration = Duration::from_millis(20);

const RUNTIME_ROOT_RELATIVE_PATH: &str = r"FyAgent\runtime";
static RUNTIME_STATUS: OnceLock<RuntimePrivilegeStatus> = OnceLock::new();
static INSTANCE_GUARD: OnceLock<Arc<InstanceGuard>> = OnceLock::new();

#[derive(Debug)]
struct PrivilegeProbe {
    user_sid: String,
    session_id: u32,
    status: RuntimePrivilegeStatus,
}

/// Kernel and file handles are process capabilities rather than pointers to
/// Rust-managed memory. Ownership is transferred exactly once, and all shared
/// access is serialized by the guard or listener thread surrounding it.
struct OwnedHandle(HANDLE);

unsafe impl Send for OwnedHandle {}

impl OwnedHandle {
    fn get(&self) -> HANDLE {
        self.0
    }
}

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        if !self.0.is_invalid() && self.0 != INVALID_HANDLE_VALUE {
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }
}

struct ThreadImpersonation {
    active: bool,
}

impl ThreadImpersonation {
    fn finish(&mut self) -> bool {
        if !self.active {
            return true;
        }

        let reverted = unsafe { RevertToSelf() }.is_ok();
        if reverted {
            self.active = false;
        }
        reverted
    }
}

impl Drop for ThreadImpersonation {
    fn drop(&mut self) {
        let _ = self.finish();
    }
}

/// Creation descriptors set their owner to the built-in Administrators group.
/// Enabling SeRestorePrivilege is required to make that assignment reliable
/// instead of silently falling back to the current UAC user SID. The previous
/// token privilege state is restored as soon as the protected object is made.
struct RestorePrivilege {
    token: OwnedHandle,
    previous: TOKEN_PRIVILEGES,
    restored: bool,
}

impl RestorePrivilege {
    /// A failed restoration is a startup failure, not a best-effort warning:
    /// the pre-Tauri process must not continue with SeRestorePrivilege left
    /// enabled. Drop retries only while the process is unwinding toward its
    /// already fail-closed exit path.
    fn restore(&mut self) -> Result<(), WindowsStartupErrorCode> {
        if self.restored {
            return Ok(());
        }

        unsafe {
            SetLastError(windows::Win32::Foundation::ERROR_SUCCESS);
        }
        let adjusted = unsafe {
            AdjustTokenPrivileges(self.token.get(), false, Some(&self.previous), 0, None, None)
        };
        let adjustment_error = unsafe { GetLastError() };
        adjusted.map_err(|_| WindowsStartupErrorCode::InstanceGuardUnavailable)?;
        if adjustment_error == ERROR_NOT_ALL_ASSIGNED {
            return Err(WindowsStartupErrorCode::InstanceGuardUnavailable);
        }
        self.restored = true;
        Ok(())
    }
}

impl Drop for RestorePrivilege {
    fn drop(&mut self) {
        if !self.restored {
            let _ = self.restore();
        }
    }
}

fn enable_restore_privilege() -> Result<RestorePrivilege, WindowsStartupErrorCode> {
    let process = unsafe { GetCurrentProcess() };
    let mut token = HANDLE::default();
    unsafe { OpenProcessToken(process, TOKEN_QUERY | TOKEN_ADJUST_PRIVILEGES, &mut token) }
        .map_err(|_| WindowsStartupErrorCode::InstanceGuardUnavailable)?;
    if token.is_invalid() {
        return Err(WindowsStartupErrorCode::InstanceGuardUnavailable);
    }
    let token = OwnedHandle(token);

    let mut luid = LUID::default();
    unsafe { LookupPrivilegeValueW(None, SE_RESTORE_NAME, &mut luid) }
        .map_err(|_| WindowsStartupErrorCode::InstanceGuardUnavailable)?;
    let requested = TOKEN_PRIVILEGES {
        PrivilegeCount: 1,
        Privileges: [LUID_AND_ATTRIBUTES {
            Luid: luid,
            Attributes: SE_PRIVILEGE_ENABLED,
        }],
    };
    let mut previous = TOKEN_PRIVILEGES::default();
    unsafe {
        SetLastError(windows::Win32::Foundation::ERROR_SUCCESS);
    }
    let adjusted = unsafe {
        AdjustTokenPrivileges(
            token.get(),
            false,
            Some(&requested),
            std::mem::size_of::<TOKEN_PRIVILEGES>() as u32,
            Some(&mut previous),
            None,
        )
    };
    let adjustment_error = unsafe { GetLastError() };
    adjusted.map_err(|_| WindowsStartupErrorCode::InstanceGuardUnavailable)?;
    if adjustment_error == ERROR_NOT_ALL_ASSIGNED {
        return Err(WindowsStartupErrorCode::InstanceGuardUnavailable);
    }

    Ok(RestorePrivilege {
        token,
        previous,
        restored: false,
    })
}

/// Used for state and lease files. It intentionally grants no access to the
/// current user SID; a split-token medium process must not learn the endpoint
/// or obtain write/delete rights to the descriptor namespace.
struct ProtectedSecurityDescriptor(PSECURITY_DESCRIPTOR);

impl ProtectedSecurityDescriptor {
    fn static_object() -> Result<Self, WindowsStartupErrorCode> {
        Self::from_sddl(PROTECTED_STATIC_OBJECT_SDDL)
    }

    fn pipe_for_user(user_sid: &str) -> Result<Self, WindowsStartupErrorCode> {
        if !is_canonical_sid(user_sid) {
            return Err(WindowsStartupErrorCode::InstanceGuardUnavailable);
        }

        // The exact SID has only read/write data access, never WRITE_DAC or
        // WRITE_OWNER. The capability handshake remains mandatory even for
        // that narrow leaked-endpoint case.
        Self::from_sddl(&format!(
            "O:BAD:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GRGW;;;{user_sid})"
        ))
    }

    fn from_sddl(sddl: &str) -> Result<Self, WindowsStartupErrorCode> {
        let sddl = wide_null(sddl);
        let mut descriptor = PSECURITY_DESCRIPTOR::default();
        unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                PCWSTR(sddl.as_ptr()),
                1,
                &mut descriptor,
                None,
            )
        }
        .map_err(|_| WindowsStartupErrorCode::InstanceGuardUnavailable)?;

        if descriptor.is_invalid() {
            return Err(WindowsStartupErrorCode::InstanceGuardUnavailable);
        }

        Ok(Self(descriptor))
    }

    fn attributes(&self) -> SECURITY_ATTRIBUTES {
        SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: self.0 .0,
            bInheritHandle: false.into(),
        }
    }
}

impl Drop for ProtectedSecurityDescriptor {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            unsafe {
                let _ = LocalFree(Some(HLOCAL(self.0 .0)));
            }
        }
    }
}

#[derive(Clone)]
struct RuntimePaths {
    container: String,
    root: String,
    lease: String,
    state: String,
}

impl RuntimePaths {
    fn for_user_session(user_sid: &str, session_id: u32) -> Result<Self, WindowsStartupErrorCode> {
        let program_data = program_data_path()?;
        let container = format!(r"{program_data}\FyAgent");
        let root = format!(r"{program_data}\{RUNTIME_ROOT_RELATIVE_PATH}");
        let key = business_instance_key(user_sid, session_id);
        Ok(Self {
            container,
            root: root.clone(),
            lease: format!(r"{root}\business-{key}.lock"),
            state: format!(r"{root}\business-{key}.state"),
        })
    }
}

struct InstanceLease {
    _root: RuntimeRoot,
    _file: OwnedHandle,
}

struct RuntimeRoot {
    // Retaining both handles makes the verified ProgramData\FyAgent parent
    // and its runtime child stable for the duration of each descriptor
    // operation. A medium split-token process cannot replace either path.
    _container: OwnedHandle,
    _runtime: OwnedHandle,
}

enum LeaseAttempt {
    Held(InstanceLease),
    Contended,
}

enum StateRead {
    Missing,
    Valid(InstanceState),
    Malformed,
}

type ActivationHandler = Arc<dyn Fn(ActivationEnvelope) + Send + Sync + 'static>;

struct InstanceGuard {
    lease: Mutex<Option<InstanceLease>>,
    paths: RuntimePaths,
    state: InstanceState,
    stop: AtomicBool,
    handler: Mutex<Option<ActivationHandler>>,
    pending: Mutex<VecDeque<ActivationEnvelope>>,
    listener_done: Mutex<Option<Receiver<()>>>,
}

impl InstanceGuard {
    fn dispatch(&self, envelope: ActivationEnvelope) -> bool {
        let handler = match self.handler.lock() {
            Ok(handler) => handler.clone(),
            Err(_) => return false,
        };

        if let Some(handler) = handler {
            return std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| handler(envelope)))
                .is_ok();
        }

        let mut pending = match self.pending.lock() {
            Ok(pending) => pending,
            Err(_) => return false,
        };
        if pending.len() >= ACTIVATION_QUEUE_LIMIT {
            return false;
        }
        pending.push_back(envelope);
        true
    }

    fn install_handler(&self, handler: ActivationHandler) -> Result<(), WindowsStartupErrorCode> {
        {
            let mut registered = self
                .handler
                .lock()
                .map_err(|_| WindowsStartupErrorCode::ActivationHandlerUnavailable)?;
            if registered.is_some() {
                return Err(WindowsStartupErrorCode::ActivationHandlerUnavailable);
            }
            *registered = Some(handler.clone());
        }

        let queued = {
            let mut pending = self
                .pending
                .lock()
                .map_err(|_| WindowsStartupErrorCode::ActivationHandlerUnavailable)?;
            pending.drain(..).collect::<Vec<_>>()
        };

        for envelope in queued {
            if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| handler(envelope))).is_err()
            {
                return Err(WindowsStartupErrorCode::ActivationHandlerUnavailable);
            }
        }

        Ok(())
    }
}

pub(super) fn early_windows_startup_gate() -> WindowsStartupDisposition {
    let formal_build = formal_windows_build();
    let probe = match probe_current_process() {
        Ok(probe) => probe,
        Err(code) if formal_build => return WindowsStartupDisposition::Blocked(code),
        // Development/test manifests intentionally remain asInvoker. They do
        // not use the protected release lease because WiX may not be present.
        // A failed status probe must not turn that observability path into a
        // startup block, and it must not fall back to a predictable IPC name.
        Err(_) => return WindowsStartupDisposition::Continue,
    };
    let _ = RUNTIME_STATUS.set(probe.status);

    if let PrivilegeGateDecision::Block(code) = evaluate_privilege_gate(formal_build, probe.status)
    {
        return WindowsStartupDisposition::Blocked(code);
    }
    if !formal_build {
        return WindowsStartupDisposition::Continue;
    }

    let paths = match RuntimePaths::for_user_session(&probe.user_sid, probe.session_id) {
        Ok(paths) => paths,
        Err(code) => return WindowsStartupDisposition::Blocked(code),
    };
    let lease = match acquire_lease(&paths) {
        Ok(lease) => lease,
        Err(code) => return WindowsStartupDisposition::Blocked(code),
    };

    match lease {
        LeaseAttempt::Held(lease) => start_or_recover_instance(paths, probe, lease),
        LeaseAttempt::Contended => forward_to_descriptor_owner(&paths),
    }
}

pub(super) fn runtime_privilege_status() -> RuntimePrivilegeStatus {
    RUNTIME_STATUS.get().copied().unwrap_or_else(|| {
        probe_current_process()
            .map(|probe| probe.status)
            .unwrap_or(RuntimePrivilegeStatus {
                platform: RuntimePrivilegePlatform::Windows,
                supported: false,
                elevated: false,
                local_administrator: false,
                interactive_user_match: InteractiveUserMatch::Unavailable,
            })
    })
}

pub(super) fn install_activation_handler<F>(handler: F) -> Result<(), WindowsStartupErrorCode>
where
    F: Fn(ActivationEnvelope) + Send + Sync + 'static,
{
    let guard = INSTANCE_GUARD
        .get()
        .ok_or(WindowsStartupErrorCode::ActivationHandlerUnavailable)?;
    guard.install_handler(Arc::new(handler))
}

/// Explicit restart removes the descriptor while the protected lease is still
/// held, then asks the live random endpoint to stop. If any step fails, the
/// lease remains held so a replacement cannot become a concurrent instance.
pub(super) fn release_instance_guard() {
    let Some(guard) = INSTANCE_GUARD.get() else {
        return;
    };

    if !remove_exact_state(&guard.paths, &guard.state) {
        return;
    }

    let deadline = Instant::now() + ACTIVATION_RELEASE_TIMEOUT;
    let stop = encode_activation_stop();
    while Instant::now() < deadline {
        if forward_frame(&guard.state, &stop) {
            break;
        }
        thread::sleep(ACTIVATION_POLL_INTERVAL);
    }

    let completed = guard
        .listener_done
        .lock()
        .ok()
        .and_then(|mut done| done.take())
        .is_some_and(|done| done.recv_timeout(ACTIVATION_RELEASE_TIMEOUT).is_ok());

    if completed {
        if let Ok(mut lease) = guard.lease.lock() {
            let _ = lease.take();
        }
    }
}

fn start_or_recover_instance(
    paths: RuntimePaths,
    probe: PrivilegeProbe,
    lease: InstanceLease,
) -> WindowsStartupDisposition {
    let descriptor = match read_state(&paths) {
        Ok(descriptor) => descriptor,
        Err(code) => return WindowsStartupDisposition::Blocked(code),
    };
    let descriptor_state = match &descriptor {
        StateRead::Missing => DescriptorReadState::Missing,
        StateRead::Valid(_) => DescriptorReadState::Valid,
        StateRead::Malformed => DescriptorReadState::Malformed,
    };
    let owner_liveness = match &descriptor {
        StateRead::Valid(state) => Some(owner_liveness(state)),
        StateRead::Missing | StateRead::Malformed => None,
    };

    match decide_descriptor_startup(DescriptorLockState::Held, descriptor_state, owner_liveness) {
        DescriptorStartupDecision::CreateNew => start_first_instance(paths, probe, lease),
        DescriptorStartupDecision::RemoveStaleThenCreate => {
            let StateRead::Valid(state) = descriptor else {
                return WindowsStartupDisposition::Blocked(
                    WindowsStartupErrorCode::InstanceGuardUnavailable,
                );
            };
            if !remove_exact_state(&paths, &state) {
                return WindowsStartupDisposition::Blocked(
                    WindowsStartupErrorCode::InstanceGuardUnavailable,
                );
            }
            start_first_instance(paths, probe, lease)
        }
        DescriptorStartupDecision::ForwardExisting
        | DescriptorStartupDecision::RetryReadOnly
        | DescriptorStartupDecision::Block => {
            WindowsStartupDisposition::Blocked(WindowsStartupErrorCode::InstanceGuardUnavailable)
        }
    }
}

fn start_first_instance(
    paths: RuntimePaths,
    probe: PrivilegeProbe,
    lease: InstanceLease,
) -> WindowsStartupDisposition {
    let (owner_pid, owner_creation_time) = match current_process_identity() {
        Ok(identity) => identity,
        Err(code) => return WindowsStartupDisposition::Blocked(code),
    };
    let state = match new_instance_state(owner_pid, owner_creation_time) {
        Ok(state) => state,
        Err(code) => return WindowsStartupDisposition::Blocked(code),
    };
    let pipe = match create_activation_pipe(&state, &probe.user_sid) {
        Ok(pipe) => pipe,
        Err(code) => return WindowsStartupDisposition::Blocked(code),
    };

    // Publishing is CREATE_NEW and happens only after a server pipe exists,
    // so a reader never learns an endpoint before it can accept a connection.
    if let Err(code) = publish_state(&paths, &state) {
        drop(pipe);
        return WindowsStartupDisposition::Blocked(code);
    }

    let (done_sender, done_receiver) = mpsc::sync_channel(1);
    let guard = Arc::new(InstanceGuard {
        lease: Mutex::new(Some(lease)),
        paths: paths.clone(),
        state: state.clone(),
        stop: AtomicBool::new(false),
        handler: Mutex::new(None),
        pending: Mutex::new(VecDeque::new()),
        listener_done: Mutex::new(Some(done_receiver)),
    });

    if INSTANCE_GUARD.set(guard.clone()).is_err() {
        let _ = remove_exact_state(&paths, &state);
        return WindowsStartupDisposition::Blocked(
            WindowsStartupErrorCode::ActivationListenerUnavailable,
        );
    }

    let listener_guard = guard.clone();
    let user_sid = probe.user_sid;
    if thread::Builder::new()
        .name("fyagent-windows-activation-v2".to_string())
        .spawn(move || {
            run_activation_listener(pipe, user_sid, state, listener_guard);
            let _ = done_sender.send(());
        })
        .is_err()
    {
        let _ = remove_exact_state(&paths, &guard.state);
        return WindowsStartupDisposition::Blocked(
            WindowsStartupErrorCode::ActivationListenerUnavailable,
        );
    }

    WindowsStartupDisposition::Continue
}

fn forward_to_descriptor_owner(paths: &RuntimePaths) -> WindowsStartupDisposition {
    let frame = match current_activation_request() {
        Ok(frame) => frame,
        Err(()) => {
            return WindowsStartupDisposition::Blocked(
                WindowsStartupErrorCode::ActivationForwardUnavailable,
            )
        }
    };
    let deadline = Instant::now()
        + ACTIVATION_CONNECT_TIMEOUT
        + DESCRIPTOR_READ_RETRY.saturating_mul(DESCRIPTOR_READ_ATTEMPTS as u32);

    for _ in 0..DESCRIPTOR_READ_ATTEMPTS {
        match read_state(paths) {
            Ok(StateRead::Valid(state)) => {
                // A connection or handshake failure never authorizes cleanup.
                // The contended client does not hold the protected lease.
                if forward_frame(&state, &frame) {
                    return WindowsStartupDisposition::ForwardedToExistingInstance;
                }
                if Instant::now() >= deadline {
                    break;
                }
            }
            Ok(StateRead::Missing | StateRead::Malformed) if Instant::now() < deadline => {}
            Ok(StateRead::Missing | StateRead::Malformed) | Err(_) => break,
        }
        thread::sleep(DESCRIPTOR_READ_RETRY);
    }

    WindowsStartupDisposition::Blocked(WindowsStartupErrorCode::ActivationForwardUnavailable)
}

fn acquire_lease(paths: &RuntimePaths) -> Result<LeaseAttempt, WindowsStartupErrorCode> {
    let root = open_runtime_root(paths)?;
    let mut restore_privilege = enable_restore_privilege()?;
    let descriptor = ProtectedSecurityDescriptor::static_object()?;
    let attributes = descriptor.attributes();
    let path = wide_null(&paths.lease);
    let file = unsafe {
        CreateFileW(
            PCWSTR(path.as_ptr()),
            (FILE_GENERIC_READ | FILE_GENERIC_WRITE).0,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            Some(&attributes),
            OPEN_ALWAYS,
            FILE_ATTRIBUTE_NORMAL | FILE_FLAG_OPEN_REPARSE_POINT,
            None,
        )
    }
    .map_err(|_| WindowsStartupErrorCode::InstanceGuardUnavailable)?;
    let file = OwnedHandle(file);
    if !is_protected_regular_file(file.get()) {
        return Err(WindowsStartupErrorCode::InstanceGuardUnavailable);
    }

    let mut overlapped = OVERLAPPED::default();
    let lock_result = unsafe {
        LockFileEx(
            file.get(),
            windows::Win32::Storage::FileSystem::LOCKFILE_EXCLUSIVE_LOCK
                | windows::Win32::Storage::FileSystem::LOCKFILE_FAIL_IMMEDIATELY,
            None,
            1,
            0,
            &mut overlapped,
        )
    };
    let lock_error = unsafe { GetLastError() };
    restore_privilege.restore()?;

    match lock_result {
        Ok(()) => Ok(LeaseAttempt::Held(InstanceLease {
            _root: root,
            _file: file,
        })),
        Err(_) if lock_error == ERROR_LOCK_VIOLATION => Ok(LeaseAttempt::Contended),
        Err(_) => Err(WindowsStartupErrorCode::InstanceGuardUnavailable),
    }
}

fn read_state(paths: &RuntimePaths) -> Result<StateRead, WindowsStartupErrorCode> {
    let _root = open_runtime_root(paths)?;
    let path = wide_null(&paths.state);
    let file = match unsafe {
        CreateFileW(
            PCWSTR(path.as_ptr()),
            FILE_GENERIC_READ.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL | FILE_FLAG_OPEN_REPARSE_POINT,
            None,
        )
    } {
        Ok(file) => OwnedHandle(file),
        Err(error) if is_transient_descriptor_absence(&error) => return Ok(StateRead::Missing),
        Err(_) => return Err(WindowsStartupErrorCode::InstanceGuardUnavailable),
    };

    if !is_protected_regular_file(file.get()) {
        return Ok(StateRead::Malformed);
    }

    let mut size = 0_i64;
    if unsafe { GetFileSizeEx(file.get(), &mut size) }.is_err()
        || size != super::INSTANCE_STATE_BYTES as i64
    {
        return Ok(StateRead::Malformed);
    }

    let mut bytes = [0_u8; super::INSTANCE_STATE_BYTES];
    let mut read = 0_u32;
    if unsafe { ReadFile(file.get(), Some(&mut bytes), Some(&mut read), None) }.is_err()
        || read != bytes.len() as u32
    {
        return Ok(StateRead::Malformed);
    }

    Ok(match decode_instance_state(&bytes) {
        Ok(state) => StateRead::Valid(state),
        Err(()) => StateRead::Malformed,
    })
}

fn publish_state(
    paths: &RuntimePaths,
    state: &InstanceState,
) -> Result<(), WindowsStartupErrorCode> {
    let _root = open_runtime_root(paths)?;
    let mut restore_privilege = enable_restore_privilege()?;
    let descriptor = ProtectedSecurityDescriptor::static_object()?;
    let attributes = descriptor.attributes();
    let path = wide_null(&paths.state);
    let file = unsafe {
        CreateFileW(
            PCWSTR(path.as_ptr()),
            FILE_GENERIC_WRITE.0,
            0,
            Some(&attributes),
            CREATE_NEW,
            FILE_ATTRIBUTE_NORMAL | FILE_FLAG_OPEN_REPARSE_POINT,
            None,
        )
    }
    .map_err(|_| WindowsStartupErrorCode::InstanceGuardUnavailable)?;
    let file = OwnedHandle(file);
    if !is_protected_regular_file(file.get()) {
        return Err(WindowsStartupErrorCode::InstanceGuardUnavailable);
    }

    let frame = encode_instance_state(state);
    if !write_exact(file.get(), frame.as_bytes())
        || unsafe { FlushFileBuffers(file.get()) }.is_err()
    {
        return Err(WindowsStartupErrorCode::InstanceGuardUnavailable);
    }
    restore_privilege.restore()?;
    Ok(())
}

/// This is intentionally called only from a code path holding the lease. It
/// opens the descriptor by handle, rejects reparse points and DACL drift, then
/// compares the exact fixed frame before marking that same handle for delete.
fn remove_exact_state(paths: &RuntimePaths, expected: &InstanceState) -> bool {
    let Ok(_root) = open_runtime_root(paths) else {
        return false;
    };
    let path = wide_null(&paths.state);
    let file = match unsafe {
        CreateFileW(
            PCWSTR(path.as_ptr()),
            (DELETE | FILE_READ_DATA | READ_CONTROL).0,
            0,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL | FILE_FLAG_OPEN_REPARSE_POINT,
            None,
        )
    } {
        Ok(file) => OwnedHandle(file),
        Err(_) => return false,
    };
    if !is_protected_regular_file(file.get()) {
        return false;
    }

    let mut size = 0_i64;
    if unsafe { GetFileSizeEx(file.get(), &mut size) }.is_err()
        || size != super::INSTANCE_STATE_BYTES as i64
    {
        return false;
    }
    let mut bytes = [0_u8; super::INSTANCE_STATE_BYTES];
    let mut read = 0_u32;
    if unsafe { ReadFile(file.get(), Some(&mut bytes), Some(&mut read), None) }.is_err()
        || read != bytes.len() as u32
        || decode_instance_state(&bytes).ok().as_ref() != Some(expected)
    {
        return false;
    }

    let disposition = FILE_DISPOSITION_INFO { DeleteFile: true };
    unsafe {
        SetFileInformationByHandle(
            file.get(),
            FileDispositionInfo,
            (&disposition as *const FILE_DISPOSITION_INFO).cast(),
            std::mem::size_of::<FILE_DISPOSITION_INFO>() as u32,
        )
    }
    .is_ok()
}

fn create_activation_pipe(
    state: &InstanceState,
    user_sid: &str,
) -> Result<OwnedHandle, WindowsStartupErrorCode> {
    let mut restore_privilege = enable_restore_privilege()?;
    let descriptor = ProtectedSecurityDescriptor::pipe_for_user(user_sid)?;
    let attributes = descriptor.attributes();
    let pipe_name = state.pipe_name();
    let pipe_name = wide_null(&pipe_name);
    let pipe = unsafe {
        CreateNamedPipeW(
            PCWSTR(pipe_name.as_ptr()),
            PIPE_ACCESS_DUPLEX,
            PIPE_TYPE_MESSAGE | PIPE_READMODE_MESSAGE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
            1,
            ACTIVATION_FRAME_BYTES as u32,
            ACTIVATION_FRAME_BYTES as u32,
            ACTIVATION_CONNECT_TIMEOUT.as_millis() as u32,
            Some(&attributes),
        )
    };
    if pipe == INVALID_HANDLE_VALUE || pipe.is_invalid() {
        return Err(WindowsStartupErrorCode::ActivationListenerUnavailable);
    }
    let pipe = OwnedHandle(pipe);
    if !is_expected_pipe_security(pipe.get(), user_sid) {
        return Err(WindowsStartupErrorCode::ActivationListenerUnavailable);
    }
    restore_privilege.restore()?;
    Ok(pipe)
}

fn run_activation_listener(
    pipe: OwnedHandle,
    user_sid: String,
    state: InstanceState,
    guard: Arc<InstanceGuard>,
) {
    loop {
        if guard.stop.load(Ordering::Acquire) {
            break;
        }
        if !connect_pipe(pipe.get()) {
            break;
        }

        let accepted = receive_authenticated_activation(pipe.get(), &user_sid, &state, &guard);
        write_response(pipe.get(), accepted);
        unsafe {
            let _ = DisconnectNamedPipe(pipe.get());
        }

        if guard.stop.load(Ordering::Acquire) {
            break;
        }
    }
}

fn connect_pipe(pipe: HANDLE) -> bool {
    match unsafe { ConnectNamedPipe(pipe, None) } {
        Ok(()) => true,
        Err(_) => (unsafe { GetLastError() }) == ERROR_PIPE_CONNECTED,
    }
}

fn receive_authenticated_activation(
    pipe: HANDLE,
    expected_user_sid: &str,
    state: &InstanceState,
    guard: &InstanceGuard,
) -> bool {
    match authenticate_pipe_client(pipe, expected_user_sid) {
        PipeClientAuthentication::Authenticated => {}
        PipeClientAuthentication::Rejected => return false,
        PipeClientAuthentication::RevertFailed => {
            guard.stop.store(true, Ordering::Release);
            return false;
        }
    }

    let deadline = Instant::now() + ACTIVATION_READ_TIMEOUT;
    let mut hello = [0_u8; HANDSHAKE_FRAME_BYTES];
    if !read_exact_before(pipe, &mut hello, deadline, guard) {
        return false;
    }
    let challenge = match decode_handshake_frame(&hello) {
        Ok(HandshakeMessage::ClientHello(challenge)) => challenge,
        _ => return false,
    };
    let proof = encode_server_proof(state.capability(), challenge);
    if !write_exact(pipe, proof.as_bytes()) {
        return false;
    }

    let mut auth = [0_u8; ACTIVATION_AUTH_FRAME_BYTES];
    if !read_exact_before(pipe, &mut auth, deadline, guard) {
        return false;
    }
    let mut activation = ActivationFrame(Box::new([0_u8; ACTIVATION_FRAME_BYTES]));
    if !read_exact_before(pipe, activation.as_mut_bytes(), deadline, guard)
        || !verify_activation_auth(state.capability(), &challenge, &activation, &auth)
    {
        return false;
    }

    match decode_activation_frame(activation.as_bytes()) {
        Ok(ActivationWireMessage::Request(envelope)) => guard.dispatch(envelope),
        Ok(ActivationWireMessage::Stop) => {
            guard.stop.store(true, Ordering::Release);
            true
        }
        Err(()) => false,
    }
}

enum PipeClientAuthentication {
    Authenticated,
    Rejected,
    RevertFailed,
}

fn authenticate_pipe_client(pipe: HANDLE, expected_user_sid: &str) -> PipeClientAuthentication {
    if unsafe { ImpersonateNamedPipeClient(pipe) }.is_err() {
        return PipeClientAuthentication::Rejected;
    }
    let mut impersonation = ThreadImpersonation { active: true };

    let mut token = HANDLE::default();
    let authenticated =
        if unsafe { OpenThreadToken(GetCurrentThread(), TOKEN_QUERY, true, &mut token) }.is_err()
            || token.is_invalid()
        {
            false
        } else {
            let token = OwnedHandle(token);
            token_user_sid(token.get())
                .map(|sid| sid == expected_user_sid)
                .unwrap_or(false)
                && token_is_elevated(token.get()).unwrap_or(false)
                && token_is_local_administrator(token.get()).unwrap_or(false)
        };

    if !impersonation.finish() {
        return PipeClientAuthentication::RevertFailed;
    }
    if authenticated {
        PipeClientAuthentication::Authenticated
    } else {
        PipeClientAuthentication::Rejected
    }
}

fn forward_frame(state: &InstanceState, frame: &ActivationFrame) -> bool {
    let pipe = match open_activation_client(&state.pipe_name()) {
        Ok(pipe) => pipe,
        Err(()) => return false,
    };
    let challenge = match random_array::<HANDSHAKE_CHALLENGE_BYTES>() {
        Some(challenge) => challenge,
        None => return false,
    };
    let hello = match encode_client_hello(challenge) {
        Ok(hello) => hello,
        Err(()) => return false,
    };
    if !write_exact(pipe.get(), hello.as_bytes()) {
        return false;
    }

    let deadline = Instant::now() + ACTIVATION_READ_TIMEOUT;
    let mut proof = [0_u8; HANDSHAKE_FRAME_BYTES];
    if !read_exact_before_client(pipe.get(), &mut proof, deadline)
        || !verify_server_proof(state.capability(), challenge, &proof)
    {
        // Do not send argv unless the endpoint first proves possession of the
        // protected descriptor capability.
        return false;
    }

    let auth = encode_activation_auth(state.capability(), &challenge, frame);
    if !write_exact(pipe.get(), auth.as_bytes()) || !write_exact(pipe.get(), frame.as_bytes()) {
        return false;
    }

    let mut response = [0_u8; ACTIVATION_RESPONSE_BYTES];
    read_exact_before_client(pipe.get(), &mut response, deadline) && response == [1]
}

fn open_activation_client(pipe_name: &str) -> Result<OwnedHandle, ()> {
    let pipe_name = wide_null(pipe_name);
    let deadline = Instant::now() + ACTIVATION_CONNECT_TIMEOUT;
    loop {
        match unsafe {
            CreateFileW(
                PCWSTR(pipe_name.as_ptr()),
                (FILE_GENERIC_READ | FILE_GENERIC_WRITE).0,
                0,
                None,
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL
                    | SECURITY_SQOS_PRESENT
                    | SECURITY_IDENTIFICATION
                    | SECURITY_EFFECTIVE_ONLY,
                None,
            )
        } {
            Ok(handle) => return Ok(OwnedHandle(handle)),
            Err(_) if Instant::now() < deadline => {
                let remaining = deadline.saturating_duration_since(Instant::now());
                let timeout = remaining.as_millis().min(u32::MAX as u128) as u32;
                if unsafe { WaitNamedPipeW(PCWSTR(pipe_name.as_ptr()), timeout) }.is_err()
                    && unsafe { GetLastError() } != ERROR_PIPE_BUSY
                {
                    return Err(());
                }
            }
            Err(_) => return Err(()),
        }
    }
}

fn read_exact_before(
    pipe: HANDLE,
    bytes: &mut [u8],
    deadline: Instant,
    guard: &InstanceGuard,
) -> bool {
    while !guard.stop.load(Ordering::Acquire) && Instant::now() < deadline {
        let mut available = 0_u32;
        if unsafe { PeekNamedPipe(pipe, None, 0, None, Some(&mut available), None) }.is_err() {
            return false;
        }
        if available < bytes.len() as u32 {
            thread::sleep(ACTIVATION_POLL_INTERVAL);
            continue;
        }
        let mut read = 0_u32;
        return unsafe { ReadFile(pipe, Some(bytes), Some(&mut read), None) }.is_ok()
            && read == bytes.len() as u32;
    }
    false
}

fn read_exact_before_client(pipe: HANDLE, bytes: &mut [u8], deadline: Instant) -> bool {
    while Instant::now() < deadline {
        let mut available = 0_u32;
        if unsafe { PeekNamedPipe(pipe, None, 0, None, Some(&mut available), None) }.is_err() {
            return false;
        }
        if available < bytes.len() as u32 {
            thread::sleep(ACTIVATION_POLL_INTERVAL);
            continue;
        }
        let mut read = 0_u32;
        return unsafe { ReadFile(pipe, Some(bytes), Some(&mut read), None) }.is_ok()
            && read == bytes.len() as u32;
    }
    false
}

fn write_exact(pipe: HANDLE, bytes: &[u8]) -> bool {
    let mut written = 0_u32;
    unsafe { WriteFile(pipe, Some(bytes), Some(&mut written), None) }.is_ok()
        && written == bytes.len() as u32
}

fn write_response(pipe: HANDLE, accepted: bool) {
    let _ = write_exact(pipe, &[u8::from(accepted)]);
}

fn current_activation_request() -> Result<ActivationFrame, ()> {
    let args = std::env::args_os()
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect();
    encode_activation_request(args)
}

fn owner_liveness(state: &InstanceState) -> OwnerLiveness {
    let process =
        match unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, state.owner_pid()) } {
            Ok(process) => OwnedHandle(process),
            Err(error) if WIN32_ERROR::from_error(&error) == Some(ERROR_INVALID_PARAMETER) => {
                return OwnerLiveness::Missing
            }
            Err(_) => return OwnerLiveness::Indeterminate,
        };
    match process_creation_time(process.get()) {
        Ok(creation) if creation == state.owner_creation_time() => OwnerLiveness::Live,
        Ok(_) => OwnerLiveness::Reused,
        Err(()) => OwnerLiveness::Indeterminate,
    }
}

fn current_process_identity() -> Result<(u32, u64), WindowsStartupErrorCode> {
    let process = unsafe { GetCurrentProcess() };
    let creation = process_creation_time(process)
        .map_err(|()| WindowsStartupErrorCode::InstanceGuardUnavailable)?;
    let process_id = unsafe { GetCurrentProcessId() };
    if process_id == 0 || creation == 0 {
        return Err(WindowsStartupErrorCode::InstanceGuardUnavailable);
    }
    Ok((process_id, creation))
}

fn process_creation_time(process: HANDLE) -> Result<u64, ()> {
    let mut creation = FILETIME::default();
    let mut exit = FILETIME::default();
    let mut kernel = FILETIME::default();
    let mut user = FILETIME::default();
    unsafe { GetProcessTimes(process, &mut creation, &mut exit, &mut kernel, &mut user) }
        .map_err(|_| ())?;
    Ok((u64::from(creation.dwHighDateTime) << 32) | u64::from(creation.dwLowDateTime))
}

fn new_instance_state(
    owner_pid: u32,
    owner_creation_time: u64,
) -> Result<InstanceState, WindowsStartupErrorCode> {
    let pipe_nonce = random_array::<PIPE_NONCE_BYTES>()
        .ok_or(WindowsStartupErrorCode::InstanceGuardUnavailable)?;
    let capability = random_array::<super::ACTIVATION_CAPABILITY_BYTES>()
        .ok_or(WindowsStartupErrorCode::InstanceGuardUnavailable)?;
    InstanceState::new(owner_pid, owner_creation_time, pipe_nonce, capability)
        .map_err(|()| WindowsStartupErrorCode::InstanceGuardUnavailable)
}

fn random_array<const N: usize>() -> Option<[u8; N]> {
    let mut bytes = [0_u8; N];
    if unsafe { BCryptGenRandom(None, &mut bytes, BCRYPT_USE_SYSTEM_PREFERRED_RNG) }.is_err()
        || bytes.iter().all(|byte| *byte == 0)
    {
        return None;
    }
    Some(bytes)
}

fn open_runtime_root(paths: &RuntimePaths) -> Result<RuntimeRoot, WindowsStartupErrorCode> {
    let container = open_protected_runtime_directory(&paths.container)?;
    let runtime = open_protected_runtime_directory(&paths.root)?;
    Ok(RuntimeRoot {
        _container: container,
        _runtime: runtime,
    })
}

fn open_protected_runtime_directory(path: &str) -> Result<OwnedHandle, WindowsStartupErrorCode> {
    let path = wide_null(path);
    let directory = unsafe {
        CreateFileW(
            PCWSTR(path.as_ptr()),
            (FILE_READ_ATTRIBUTES | READ_CONTROL).0,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            None,
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
            None,
        )
    }
    .map_err(|_| WindowsStartupErrorCode::InstanceGuardUnavailable)?;
    let directory = OwnedHandle(directory);
    if !is_expected_runtime_root(directory.get()) {
        return Err(WindowsStartupErrorCode::InstanceGuardUnavailable);
    }
    Ok(directory)
}

fn is_expected_runtime_root(handle: HANDLE) -> bool {
    let mut attributes = FILE_ATTRIBUTE_TAG_INFO::default();
    unsafe {
        GetFileInformationByHandleEx(
            handle,
            FileAttributeTagInfo,
            (&mut attributes as *mut FILE_ATTRIBUTE_TAG_INFO).cast(),
            std::mem::size_of::<FILE_ATTRIBUTE_TAG_INFO>() as u32,
        )
    }
    .is_ok()
        && attributes.FileAttributes & FILE_ATTRIBUTE_DIRECTORY.0 != 0
        && attributes.FileAttributes & FILE_ATTRIBUTE_REPARSE_POINT.0 == 0
        && root_security_matches(handle)
}

fn is_protected_regular_file(handle: HANDLE) -> bool {
    let mut attributes = FILE_ATTRIBUTE_TAG_INFO::default();
    unsafe {
        GetFileInformationByHandleEx(
            handle,
            FileAttributeTagInfo,
            (&mut attributes as *mut FILE_ATTRIBUTE_TAG_INFO).cast(),
            std::mem::size_of::<FILE_ATTRIBUTE_TAG_INFO>() as u32,
        )
    }
    .is_ok()
        && attributes.FileAttributes & FILE_ATTRIBUTE_DIRECTORY.0 == 0
        && attributes.FileAttributes & FILE_ATTRIBUTE_REPARSE_POINT.0 == 0
        && static_object_security_matches(handle)
}

fn root_security_matches(handle: HANDLE) -> bool {
    let Some(sddl) = security_sddl(
        handle,
        windows::Win32::Security::OWNER_SECURITY_INFORMATION
            | windows::Win32::Security::DACL_SECURITY_INFORMATION,
    ) else {
        return false;
    };
    is_expected_runtime_root_sddl(&sddl)
}

fn static_object_security_matches(handle: HANDLE) -> bool {
    let Some(sddl) = security_sddl(
        handle,
        windows::Win32::Security::OWNER_SECURITY_INFORMATION
            | windows::Win32::Security::DACL_SECURITY_INFORMATION,
    ) else {
        return false;
    };
    is_expected_static_object_sddl(&sddl)
}

fn is_expected_pipe_security(handle: HANDLE, user_sid: &str) -> bool {
    let Some(sddl) = security_sddl(
        handle,
        windows::Win32::Security::OWNER_SECURITY_INFORMATION
            | windows::Win32::Security::DACL_SECURITY_INFORMATION,
    ) else {
        return false;
    };
    is_expected_pipe_sddl(&sddl, user_sid)
}

fn security_sddl(
    handle: HANDLE,
    information: windows::Win32::Security::OBJECT_SECURITY_INFORMATION,
) -> Option<String> {
    let mut descriptor = PSECURITY_DESCRIPTOR::default();
    let security_result = unsafe {
        GetSecurityInfo(
            handle,
            windows::Win32::Security::Authorization::SE_FILE_OBJECT,
            information,
            None,
            None,
            None,
            None,
            Some(&mut descriptor),
        )
    };
    if security_result.is_err() || descriptor.is_invalid() {
        return None;
    }

    let mut text = PWSTR::null();
    let converted = unsafe {
        ConvertSecurityDescriptorToStringSecurityDescriptorW(
            descriptor,
            1,
            information,
            &mut text,
            None,
        )
    };
    unsafe {
        let _ = LocalFree(Some(HLOCAL(descriptor.0)));
    }
    if converted.is_err() || text.is_null() {
        return None;
    }
    let result = unsafe { text.to_string() }.ok();
    unsafe {
        let _ = LocalFree(Some(HLOCAL(text.0.cast())));
    }
    result
}

fn program_data_path() -> Result<String, WindowsStartupErrorCode> {
    let path = unsafe { SHGetKnownFolderPath(&FOLDERID_ProgramData, KNOWN_FOLDER_FLAG(0), None) }
        .map_err(|_| WindowsStartupErrorCode::InstanceGuardUnavailable)?;
    if path.is_null() {
        return Err(WindowsStartupErrorCode::InstanceGuardUnavailable);
    }
    let value =
        unsafe { path.to_string() }.map_err(|_| WindowsStartupErrorCode::InstanceGuardUnavailable);
    unsafe {
        CoTaskMemFree(Some(path.0.cast()));
    }
    value
}

fn is_transient_descriptor_absence(error: &windows::core::Error) -> bool {
    matches!(
        WIN32_ERROR::from_error(error),
        Some(ERROR_FILE_NOT_FOUND | ERROR_SHARING_VIOLATION)
    )
}

fn probe_current_process() -> Result<PrivilegeProbe, WindowsStartupErrorCode> {
    let token = current_process_token()?;
    let user_sid = token_user_sid(token.get())?;
    let session_id = token_session_id(token.get())?;
    let elevated = token_is_elevated(token.get())?;
    let local_administrator = token_is_local_administrator(token.get())?;
    let interactive_user_match = match interactive_session_user_sid(session_id)
        .or_else(|| shell_window_user_sid(session_id))
    {
        Some(interactive_sid) if interactive_sid == user_sid => InteractiveUserMatch::Match,
        Some(_) => InteractiveUserMatch::Mismatch,
        None => InteractiveUserMatch::Unavailable,
    };

    Ok(PrivilegeProbe {
        user_sid,
        session_id,
        status: RuntimePrivilegeStatus {
            platform: RuntimePrivilegePlatform::Windows,
            supported: true,
            elevated,
            local_administrator,
            interactive_user_match,
        },
    })
}

fn current_process_token() -> Result<OwnedHandle, WindowsStartupErrorCode> {
    let process = unsafe { GetCurrentProcess() };
    open_process_token(process)
}

fn open_process_token(process: HANDLE) -> Result<OwnedHandle, WindowsStartupErrorCode> {
    let mut token = HANDLE::default();
    unsafe { OpenProcessToken(process, TOKEN_QUERY, &mut token) }
        .map_err(|_| WindowsStartupErrorCode::PrivilegeStatusUnavailable)?;
    if token.is_invalid() {
        return Err(WindowsStartupErrorCode::PrivilegeStatusUnavailable);
    }
    Ok(OwnedHandle(token))
}

fn token_user_sid(token: HANDLE) -> Result<String, WindowsStartupErrorCode> {
    let mut required = 0_u32;
    let _ = unsafe { GetTokenInformation(token, TokenUser, None, 0, &mut required) };
    if required < std::mem::size_of::<TOKEN_USER>() as u32 {
        return Err(WindowsStartupErrorCode::PrivilegeStatusUnavailable);
    }

    let mut buffer = vec![0_u8; required as usize];
    unsafe {
        GetTokenInformation(
            token,
            TokenUser,
            Some(buffer.as_mut_ptr().cast()),
            required,
            &mut required,
        )
    }
    .map_err(|_| WindowsStartupErrorCode::PrivilegeStatusUnavailable)?;
    let token_user = unsafe { &*(buffer.as_ptr().cast::<TOKEN_USER>()) };
    if token_user.User.Sid.is_invalid() {
        return Err(WindowsStartupErrorCode::PrivilegeStatusUnavailable);
    }
    sid_to_string(token_user.User.Sid)
}

fn token_session_id(token: HANDLE) -> Result<u32, WindowsStartupErrorCode> {
    let mut session_id = 0_u32;
    let mut returned = 0_u32;
    unsafe {
        GetTokenInformation(
            token,
            TokenSessionId,
            Some((&mut session_id as *mut u32).cast()),
            std::mem::size_of::<u32>() as u32,
            &mut returned,
        )
    }
    .map_err(|_| WindowsStartupErrorCode::PrivilegeStatusUnavailable)?;
    if returned != std::mem::size_of::<u32>() as u32 {
        return Err(WindowsStartupErrorCode::PrivilegeStatusUnavailable);
    }
    Ok(session_id)
}

fn token_is_elevated(token: HANDLE) -> Result<bool, WindowsStartupErrorCode> {
    let mut elevation = TOKEN_ELEVATION::default();
    let mut returned = 0_u32;
    unsafe {
        GetTokenInformation(
            token,
            TokenElevation,
            Some((&mut elevation as *mut TOKEN_ELEVATION).cast()),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut returned,
        )
    }
    .map_err(|_| WindowsStartupErrorCode::PrivilegeStatusUnavailable)?;
    if returned != std::mem::size_of::<TOKEN_ELEVATION>() as u32 {
        return Err(WindowsStartupErrorCode::PrivilegeStatusUnavailable);
    }
    Ok(elevation.TokenIsElevated != 0)
}

fn token_is_local_administrator(token: HANDLE) -> Result<bool, WindowsStartupErrorCode> {
    let mut sid_buffer = [0_u8; SECURITY_MAX_SID_SIZE as usize];
    let mut sid_size = sid_buffer.len() as u32;
    let sid = PSID(sid_buffer.as_mut_ptr().cast());
    unsafe { CreateWellKnownSid(WinBuiltinAdministratorsSid, None, Some(sid), &mut sid_size) }
        .map_err(|_| WindowsStartupErrorCode::PrivilegeStatusUnavailable)?;

    let mut member = false.into();
    unsafe { CheckTokenMembership(Some(token), sid, &mut member) }
        .map_err(|_| WindowsStartupErrorCode::PrivilegeStatusUnavailable)?;
    Ok(bool::from(member))
}

fn interactive_session_user_sid(session_id: u32) -> Option<String> {
    let mut token = HANDLE::default();
    if unsafe { WTSQueryUserToken(session_id, &mut token) }.is_err() || token.is_invalid() {
        return None;
    }
    let token = OwnedHandle(token);
    token_user_sid(token.get()).ok()
}

fn shell_window_user_sid(expected_session_id: u32) -> Option<String> {
    let shell_window = unsafe { GetShellWindow() };
    if shell_window.is_invalid() {
        return None;
    }

    let mut process_id = 0_u32;
    if unsafe { GetWindowThreadProcessId(shell_window, Some(&mut process_id)) } == 0
        || process_id == 0
    {
        return None;
    }

    let mut shell_session_id = 0_u32;
    if unsafe { ProcessIdToSessionId(process_id, &mut shell_session_id) }.is_err()
        || shell_session_id != expected_session_id
    {
        return None;
    }

    let process =
        unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id) }.ok()?;
    let process = OwnedHandle(process);
    let token = open_process_token(process.get()).ok()?;
    token_user_sid(token.get()).ok()
}

fn sid_to_string(sid: PSID) -> Result<String, WindowsStartupErrorCode> {
    let mut string_sid = PWSTR::null();
    unsafe { ConvertSidToStringSidW(sid, &mut string_sid) }
        .map_err(|_| WindowsStartupErrorCode::PrivilegeStatusUnavailable)?;
    if string_sid.is_null() {
        return Err(WindowsStartupErrorCode::PrivilegeStatusUnavailable);
    }

    let value = unsafe { string_sid.to_string() }
        .map_err(|_| WindowsStartupErrorCode::PrivilegeStatusUnavailable);
    unsafe {
        let _ = LocalFree(Some(HLOCAL(string_sid.0.cast())));
    }
    let value = value?;
    if !is_canonical_sid(&value) {
        return Err(WindowsStartupErrorCode::PrivilegeStatusUnavailable);
    }
    Ok(value)
}

fn is_canonical_sid(value: &str) -> bool {
    value.starts_with("S-")
        && value.len() <= 184
        && value
            .bytes()
            .all(|byte| byte == b'S' || byte == b'-' || byte.is_ascii_digit())
}

fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

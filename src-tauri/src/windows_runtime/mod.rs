//! Windows early-runtime guards that must run before Tauri creates a runtime.
//!
//! The platform-neutral protocol and policy checks live here so they can be
//! unit-tested without touching a Windows token, named pipe, process, or
//! registry. Native Win32 calls are isolated in `native.rs`.

use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const ACTIVATION_MAGIC: [u8; 8] = *b"FYACTV1\0";
const ACTIVATION_PROTOCOL_VERSION: u8 = 1;
const ACTIVATION_KIND_REQUEST: u8 = 1;
const ACTIVATION_KIND_STOP: u8 = 2;
const ACTIVATION_HEADER_BYTES: usize = 16;
/// The deep-link parser accepts a 64 KiB URL. Leave deterministic room for
/// its argv envelope and JSON schema while retaining a fixed upper bound.
pub(crate) const ACTIVATION_FRAME_BYTES: usize = 72 * 1024;
const MAX_ACTIVATION_ARGUMENTS: usize = 8;
const MAX_ACTIVATION_ARGUMENT_BYTES: usize = 64 * 1024;

/// Release builds publish this fixed-size descriptor only inside the WiX-owned
/// ProgramData runtime root. The state file names are deterministic (from a
/// SID/session hash), while every live pipe endpoint is a fresh secret.
const INSTANCE_STATE_MAGIC: [u8; 8] = *b"FYAGST2\0";
const INSTANCE_STATE_VERSION: u8 = 2;
const INSTANCE_STATE_BYTES: usize = 96;
pub(crate) const PIPE_NONCE_BYTES: usize = 31;
pub(crate) const ACTIVATION_CAPABILITY_BYTES: usize = 32;

const HANDSHAKE_MAGIC: [u8; 8] = *b"FYAGHS2\0";
const HANDSHAKE_VERSION: u8 = 2;
const HANDSHAKE_CLIENT_HELLO: u8 = 1;
const HANDSHAKE_SERVER_PROOF: u8 = 2;
pub(crate) const HANDSHAKE_FRAME_BYTES: usize = 80;
pub(crate) const HANDSHAKE_CHALLENGE_BYTES: usize = 32;
pub(crate) const ACTIVATION_AUTH_TAG_BYTES: usize = 32;
pub(crate) const ACTIVATION_AUTH_FRAME_BYTES: usize = 44;

const SERVER_PROOF_DOMAIN: &[u8] = b"fyagent/windows-activation/v2/server-proof\0";
const REQUEST_AUTH_DOMAIN: &[u8] = b"fyagent/windows-activation/v2/request-auth\0";

/// State and lease files must never be owned by a user SID. In a UAC
/// split-token session, that owner SID is present in the medium token and can
/// imply WRITE_DAC even when the visible DACL is otherwise restrictive.
#[cfg(target_os = "windows")]
pub(crate) const PROTECTED_STATIC_OBJECT_SDDL: &str = "O:BAD:P(A;;FA;;;SY)(A;;FA;;;BA)";

type HmacSha256 = Hmac<Sha256>;

/// Only safe, renderer-facing facts about the current process are exposed.
/// In particular this deliberately excludes SIDs, account names, token
/// handles, paths, and raw Windows error information.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimePrivilegeStatus {
    pub platform: RuntimePrivilegePlatform,
    pub supported: bool,
    pub elevated: bool,
    pub local_administrator: bool,
    pub interactive_user_match: InteractiveUserMatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RuntimePrivilegePlatform {
    Windows,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum InteractiveUserMatch {
    Match,
    Mismatch,
    Unavailable,
}

impl RuntimePrivilegeStatus {
    const fn unsupported() -> Self {
        Self {
            platform: RuntimePrivilegePlatform::Other,
            supported: false,
            elevated: false,
            local_administrator: false,
            interactive_user_match: InteractiveUserMatch::Unavailable,
        }
    }
}

/// Stable codes are intentionally the only pre-logger diagnostics. They are
/// safe to print before the user-configured log sink exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowsStartupErrorCode {
    PrivilegeStatusUnavailable,
    RequiredElevationMissing,
    RequiredLocalAdministratorMissing,
    InteractiveUserMismatch,
    InteractiveUserUnavailable,
    InstanceGuardUnavailable,
    ActivationForwardUnavailable,
    ActivationListenerUnavailable,
    ActivationHandlerUnavailable,
}

impl WindowsStartupErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PrivilegeStatusUnavailable => "WIN_PRIVILEGE_STATUS_UNAVAILABLE",
            Self::RequiredElevationMissing => "WIN_REQUIRED_ELEVATION_MISSING",
            Self::RequiredLocalAdministratorMissing => "WIN_REQUIRED_LOCAL_ADMIN_MISSING",
            Self::InteractiveUserMismatch => "WIN_INTERACTIVE_USER_MISMATCH",
            Self::InteractiveUserUnavailable => "WIN_INTERACTIVE_USER_UNAVAILABLE",
            Self::InstanceGuardUnavailable => "WIN_INSTANCE_GUARD_UNAVAILABLE",
            Self::ActivationForwardUnavailable => "WIN_INSTANCE_ACTIVATION_UNAVAILABLE",
            Self::ActivationListenerUnavailable => "WIN_INSTANCE_LISTENER_UNAVAILABLE",
            Self::ActivationHandlerUnavailable => "WIN_INSTANCE_HANDLER_UNAVAILABLE",
        }
    }
}

impl std::fmt::Display for WindowsStartupErrorCode {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// The only outcomes accepted by `main` before Tauri construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowsStartupDisposition {
    Continue,
    ForwardedToExistingInstance,
    Blocked(WindowsStartupErrorCode),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PrivilegeGateDecision {
    Allow,
    Block(WindowsStartupErrorCode),
}

/// `fyagent_windows_release` is supplied only by the release manifest build
/// path. Development and test binaries deliberately use `asInvoker`, so a
/// non-elevated status must remain observable rather than blocking startup.
#[cfg(target_os = "windows")]
pub(crate) const fn formal_windows_build() -> bool {
    cfg!(all(target_os = "windows", fyagent_windows_release))
}

fn evaluate_privilege_gate(
    formal_build: bool,
    status: RuntimePrivilegeStatus,
) -> PrivilegeGateDecision {
    if !formal_build {
        return PrivilegeGateDecision::Allow;
    }

    if !status.supported {
        return PrivilegeGateDecision::Block(WindowsStartupErrorCode::PrivilegeStatusUnavailable);
    }
    if !status.elevated {
        return PrivilegeGateDecision::Block(WindowsStartupErrorCode::RequiredElevationMissing);
    }
    if !status.local_administrator {
        return PrivilegeGateDecision::Block(
            WindowsStartupErrorCode::RequiredLocalAdministratorMissing,
        );
    }
    match status.interactive_user_match {
        InteractiveUserMatch::Match => {}
        InteractiveUserMatch::Mismatch => {
            return PrivilegeGateDecision::Block(WindowsStartupErrorCode::InteractiveUserMismatch)
        }
        // A release binary is intentionally stricter than the asInvoker
        // development/test manifest: absence of an identity proof is not a
        // proof that the elevated process belongs to the interactive user.
        InteractiveUserMatch::Unavailable => {
            return PrivilegeGateDecision::Block(
                WindowsStartupErrorCode::InteractiveUserUnavailable,
            )
        }
    }

    PrivilegeGateDecision::Allow
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActivationEnvelope {
    args: Vec<String>,
}

impl ActivationEnvelope {
    #[cfg(target_os = "windows")]
    pub(crate) fn args(&self) -> &[String] {
        &self.args
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActivationFrame(Box<[u8; ACTIVATION_FRAME_BYTES]>);

impl ActivationFrame {
    pub(crate) fn as_bytes(&self) -> &[u8] {
        &self.0[..]
    }

    pub(crate) fn as_mut_bytes(&mut self) -> &mut [u8] {
        &mut self.0[..]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ActivationWireMessage {
    Request(ActivationEnvelope),
    Stop,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ActivationPayload {
    version: u8,
    args: Vec<String>,
}

fn validate_activation_args(args: &[String]) -> Result<(), ()> {
    if args.len() > MAX_ACTIVATION_ARGUMENTS {
        return Err(());
    }

    if args.iter().any(|argument| {
        argument.len() > MAX_ACTIVATION_ARGUMENT_BYTES || argument.chars().any(char::is_control)
    }) {
        return Err(());
    }

    Ok(())
}

pub(crate) fn encode_activation_request(args: Vec<String>) -> Result<ActivationFrame, ()> {
    validate_activation_args(&args)?;

    let payload = serde_json::to_vec(&serde_json::json!({
        "version": ACTIVATION_PROTOCOL_VERSION,
        "args": args,
    }))
    .map_err(|_| ())?;
    let capacity = ACTIVATION_FRAME_BYTES - ACTIVATION_HEADER_BYTES;
    if payload.len() > capacity || payload.len() > u32::MAX as usize {
        return Err(());
    }

    let mut frame = Box::new([0_u8; ACTIVATION_FRAME_BYTES]);
    frame[..ACTIVATION_MAGIC.len()].copy_from_slice(&ACTIVATION_MAGIC);
    frame[8] = ACTIVATION_PROTOCOL_VERSION;
    frame[9] = ACTIVATION_KIND_REQUEST;
    frame[10..14].copy_from_slice(&(payload.len() as u32).to_le_bytes());
    frame[ACTIVATION_HEADER_BYTES..ACTIVATION_HEADER_BYTES + payload.len()]
        .copy_from_slice(&payload);
    Ok(ActivationFrame(frame))
}

pub(crate) fn encode_activation_stop() -> ActivationFrame {
    let mut frame = Box::new([0_u8; ACTIVATION_FRAME_BYTES]);
    frame[..ACTIVATION_MAGIC.len()].copy_from_slice(&ACTIVATION_MAGIC);
    frame[8] = ACTIVATION_PROTOCOL_VERSION;
    frame[9] = ACTIVATION_KIND_STOP;
    ActivationFrame(frame)
}

pub(crate) fn decode_activation_frame(frame: &[u8]) -> Result<ActivationWireMessage, ()> {
    if frame.len() != ACTIVATION_FRAME_BYTES
        || frame[..ACTIVATION_MAGIC.len()] != ACTIVATION_MAGIC
        || frame[8] != ACTIVATION_PROTOCOL_VERSION
        || frame[14..ACTIVATION_HEADER_BYTES]
            .iter()
            .any(|byte| *byte != 0)
    {
        return Err(());
    }

    let payload_length = u32::from_le_bytes(
        frame[10..14]
            .try_into()
            .expect("activation frame header has a fixed length"),
    ) as usize;
    let payload_end = ACTIVATION_HEADER_BYTES
        .checked_add(payload_length)
        .filter(|end| *end <= ACTIVATION_FRAME_BYTES)
        .ok_or(())?;

    if frame[payload_end..].iter().any(|byte| *byte != 0) {
        return Err(());
    }

    match frame[9] {
        ACTIVATION_KIND_STOP if payload_length == 0 => Ok(ActivationWireMessage::Stop),
        ACTIVATION_KIND_REQUEST => {
            let payload: ActivationPayload =
                serde_json::from_slice(&frame[ACTIVATION_HEADER_BYTES..payload_end])
                    .map_err(|_| ())?;
            if payload.version != ACTIVATION_PROTOCOL_VERSION {
                return Err(());
            }
            validate_activation_args(&payload.args)?;
            Ok(ActivationWireMessage::Request(ActivationEnvelope {
                args: payload.args,
            }))
        }
        _ => Err(()),
    }
}

/// A capability-bearing descriptor for one live elevated business instance.
/// It is intentionally not serializable or debuggable: its nonce and
/// capability must never become renderer data, diagnostics, or object names
/// derived from predictable user information.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct InstanceState {
    owner_pid: u32,
    owner_creation_time: u64,
    pipe_nonce: [u8; PIPE_NONCE_BYTES],
    capability: [u8; ACTIVATION_CAPABILITY_BYTES],
}

impl InstanceState {
    pub(crate) fn new(
        owner_pid: u32,
        owner_creation_time: u64,
        pipe_nonce: [u8; PIPE_NONCE_BYTES],
        capability: [u8; ACTIVATION_CAPABILITY_BYTES],
    ) -> Result<Self, ()> {
        if owner_pid == 0
            || owner_creation_time == 0
            || pipe_nonce.iter().all(|byte| *byte == 0)
            || capability.iter().all(|byte| *byte == 0)
        {
            return Err(());
        }

        Ok(Self {
            owner_pid,
            owner_creation_time,
            pipe_nonce,
            capability,
        })
    }

    pub(crate) const fn owner_pid(&self) -> u32 {
        self.owner_pid
    }

    pub(crate) const fn owner_creation_time(&self) -> u64 {
        self.owner_creation_time
    }

    pub(crate) fn pipe_name(&self) -> String {
        let mut endpoint = String::from(r"\\.\pipe\FyAgent.Activation.v2.");
        // A 31-byte nonce is rendered as 61 hexadecimal nibbles. This gives
        // 244 bits of endpoint entropy while keeping the protocol invariant
        // explicit instead of relying on a variable UUID implementation.
        for (index, byte) in self.pipe_nonce.iter().enumerate() {
            if index == PIPE_NONCE_BYTES - 1 {
                endpoint.push_str(&format!("{:x}", byte >> 4));
            } else {
                endpoint.push_str(&format!("{byte:02x}"));
            }
        }
        endpoint
    }

    pub(crate) fn capability(&self) -> &[u8; ACTIVATION_CAPABILITY_BYTES] {
        &self.capability
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct InstanceStateFrame(Box<[u8; INSTANCE_STATE_BYTES]>);

impl InstanceStateFrame {
    pub(crate) fn as_bytes(&self) -> &[u8] {
        &self.0[..]
    }
}

pub(crate) fn encode_instance_state(state: &InstanceState) -> InstanceStateFrame {
    let mut frame = Box::new([0_u8; INSTANCE_STATE_BYTES]);
    frame[..INSTANCE_STATE_MAGIC.len()].copy_from_slice(&INSTANCE_STATE_MAGIC);
    frame[8] = INSTANCE_STATE_VERSION;
    frame[12..16].copy_from_slice(&state.owner_pid.to_le_bytes());
    frame[16..24].copy_from_slice(&state.owner_creation_time.to_le_bytes());
    frame[24..24 + PIPE_NONCE_BYTES].copy_from_slice(&state.pipe_nonce);
    frame[24 + PIPE_NONCE_BYTES..24 + PIPE_NONCE_BYTES + ACTIVATION_CAPABILITY_BYTES]
        .copy_from_slice(&state.capability);
    InstanceStateFrame(frame)
}

pub(crate) fn decode_instance_state(frame: &[u8]) -> Result<InstanceState, ()> {
    if frame.len() != INSTANCE_STATE_BYTES
        || frame[..INSTANCE_STATE_MAGIC.len()] != INSTANCE_STATE_MAGIC
        || frame[8] != INSTANCE_STATE_VERSION
        || frame[9..12].iter().any(|byte| *byte != 0)
        || frame[24 + PIPE_NONCE_BYTES + ACTIVATION_CAPABILITY_BYTES..]
            .iter()
            .any(|byte| *byte != 0)
    {
        return Err(());
    }

    let owner_pid = u32::from_le_bytes(frame[12..16].try_into().map_err(|_| ())?);
    let owner_creation_time = u64::from_le_bytes(frame[16..24].try_into().map_err(|_| ())?);
    let mut pipe_nonce = [0_u8; PIPE_NONCE_BYTES];
    pipe_nonce.copy_from_slice(&frame[24..24 + PIPE_NONCE_BYTES]);
    let mut capability = [0_u8; ACTIVATION_CAPABILITY_BYTES];
    capability.copy_from_slice(
        &frame[24 + PIPE_NONCE_BYTES..24 + PIPE_NONCE_BYTES + ACTIVATION_CAPABILITY_BYTES],
    );
    InstanceState::new(owner_pid, owner_creation_time, pipe_nonce, capability)
}

/// V2 uses a challenge-response before argv is sent. A client learns whether
/// it reached the descriptor owner before it transmits a deep link, and the
/// server later verifies that the client also knows the protected capability.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct HandshakeFrame([u8; HANDSHAKE_FRAME_BYTES]);

impl HandshakeFrame {
    pub(crate) fn as_bytes(&self) -> &[u8] {
        &self.0[..]
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) enum HandshakeMessage {
    ClientHello([u8; HANDSHAKE_CHALLENGE_BYTES]),
    ServerProof {
        challenge: [u8; HANDSHAKE_CHALLENGE_BYTES],
        tag: [u8; ACTIVATION_AUTH_TAG_BYTES],
    },
}

pub(crate) fn encode_client_hello(
    challenge: [u8; HANDSHAKE_CHALLENGE_BYTES],
) -> Result<HandshakeFrame, ()> {
    if challenge.iter().all(|byte| *byte == 0) {
        return Err(());
    }

    let mut frame = [0_u8; HANDSHAKE_FRAME_BYTES];
    frame[..HANDSHAKE_MAGIC.len()].copy_from_slice(&HANDSHAKE_MAGIC);
    frame[8] = HANDSHAKE_VERSION;
    frame[9] = HANDSHAKE_CLIENT_HELLO;
    frame[12..12 + HANDSHAKE_CHALLENGE_BYTES].copy_from_slice(&challenge);
    Ok(HandshakeFrame(frame))
}

pub(crate) fn encode_server_proof(
    capability: &[u8; ACTIVATION_CAPABILITY_BYTES],
    challenge: [u8; HANDSHAKE_CHALLENGE_BYTES],
) -> HandshakeFrame {
    let mut frame = [0_u8; HANDSHAKE_FRAME_BYTES];
    frame[..HANDSHAKE_MAGIC.len()].copy_from_slice(&HANDSHAKE_MAGIC);
    frame[8] = HANDSHAKE_VERSION;
    frame[9] = HANDSHAKE_SERVER_PROOF;
    frame[12..12 + HANDSHAKE_CHALLENGE_BYTES].copy_from_slice(&challenge);
    let tag = server_proof_tag(capability, &challenge);
    frame[12 + HANDSHAKE_CHALLENGE_BYTES
        ..12 + HANDSHAKE_CHALLENGE_BYTES + ACTIVATION_AUTH_TAG_BYTES]
        .copy_from_slice(&tag);
    HandshakeFrame(frame)
}

pub(crate) fn decode_handshake_frame(frame: &[u8]) -> Result<HandshakeMessage, ()> {
    if frame.len() != HANDSHAKE_FRAME_BYTES
        || frame[..HANDSHAKE_MAGIC.len()] != HANDSHAKE_MAGIC
        || frame[8] != HANDSHAKE_VERSION
        || frame[10..12].iter().any(|byte| *byte != 0)
        || frame[12 + HANDSHAKE_CHALLENGE_BYTES + ACTIVATION_AUTH_TAG_BYTES..]
            .iter()
            .any(|byte| *byte != 0)
    {
        return Err(());
    }

    let mut challenge = [0_u8; HANDSHAKE_CHALLENGE_BYTES];
    challenge.copy_from_slice(&frame[12..12 + HANDSHAKE_CHALLENGE_BYTES]);
    if challenge.iter().all(|byte| *byte == 0) {
        return Err(());
    }

    let tag_range =
        12 + HANDSHAKE_CHALLENGE_BYTES..12 + HANDSHAKE_CHALLENGE_BYTES + ACTIVATION_AUTH_TAG_BYTES;
    let mut tag = [0_u8; ACTIVATION_AUTH_TAG_BYTES];
    tag.copy_from_slice(&frame[tag_range]);

    match frame[9] {
        HANDSHAKE_CLIENT_HELLO if tag.iter().all(|byte| *byte == 0) => {
            Ok(HandshakeMessage::ClientHello(challenge))
        }
        HANDSHAKE_SERVER_PROOF if !tag.iter().all(|byte| *byte == 0) => {
            Ok(HandshakeMessage::ServerProof { challenge, tag })
        }
        _ => Err(()),
    }
}

pub(crate) fn verify_server_proof(
    capability: &[u8; ACTIVATION_CAPABILITY_BYTES],
    challenge: [u8; HANDSHAKE_CHALLENGE_BYTES],
    proof: &[u8],
) -> bool {
    match decode_handshake_frame(proof) {
        Ok(HandshakeMessage::ServerProof {
            challenge: returned_challenge,
            tag,
        }) if returned_challenge == challenge => verify_hmac(
            capability,
            SERVER_PROOF_DOMAIN,
            &[&returned_challenge],
            &tag,
        ),
        _ => false,
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct ActivationAuthFrame([u8; ACTIVATION_AUTH_FRAME_BYTES]);

impl ActivationAuthFrame {
    pub(crate) fn as_bytes(&self) -> &[u8] {
        &self.0[..]
    }
}

pub(crate) fn encode_activation_auth(
    capability: &[u8; ACTIVATION_CAPABILITY_BYTES],
    challenge: &[u8; HANDSHAKE_CHALLENGE_BYTES],
    activation: &ActivationFrame,
) -> ActivationAuthFrame {
    let mut frame = [0_u8; ACTIVATION_AUTH_FRAME_BYTES];
    frame[..8].copy_from_slice(b"FYAGAU2\0");
    frame[8] = HANDSHAKE_VERSION;
    let tag = request_auth_tag(capability, challenge, activation.as_bytes());
    frame[12..].copy_from_slice(&tag);
    ActivationAuthFrame(frame)
}

pub(crate) fn verify_activation_auth(
    capability: &[u8; ACTIVATION_CAPABILITY_BYTES],
    challenge: &[u8; HANDSHAKE_CHALLENGE_BYTES],
    activation: &ActivationFrame,
    auth: &[u8],
) -> bool {
    if auth.len() != ACTIVATION_AUTH_FRAME_BYTES
        || auth[..8] != *b"FYAGAU2\0"
        || auth[8] != HANDSHAKE_VERSION
        || auth[9..12].iter().any(|byte| *byte != 0)
    {
        return false;
    }

    verify_hmac(
        capability,
        REQUEST_AUTH_DOMAIN,
        &[challenge, activation.as_bytes()],
        &auth[12..],
    )
}

fn server_proof_tag(
    capability: &[u8; ACTIVATION_CAPABILITY_BYTES],
    challenge: &[u8; HANDSHAKE_CHALLENGE_BYTES],
) -> [u8; ACTIVATION_AUTH_TAG_BYTES] {
    hmac_tag(capability, SERVER_PROOF_DOMAIN, &[challenge])
}

fn request_auth_tag(
    capability: &[u8; ACTIVATION_CAPABILITY_BYTES],
    challenge: &[u8; HANDSHAKE_CHALLENGE_BYTES],
    activation: &[u8],
) -> [u8; ACTIVATION_AUTH_TAG_BYTES] {
    hmac_tag(capability, REQUEST_AUTH_DOMAIN, &[challenge, activation])
}

fn hmac_tag(
    capability: &[u8; ACTIVATION_CAPABILITY_BYTES],
    domain: &[u8],
    chunks: &[&[u8]],
) -> [u8; ACTIVATION_AUTH_TAG_BYTES] {
    let mut mac = HmacSha256::new_from_slice(capability)
        .expect("HMAC-SHA-256 accepts the fixed-length activation capability");
    mac.update(domain);
    for chunk in chunks {
        mac.update(chunk);
    }
    let mut tag = [0_u8; ACTIVATION_AUTH_TAG_BYTES];
    tag.copy_from_slice(&mac.finalize().into_bytes());
    tag
}

fn verify_hmac(
    capability: &[u8; ACTIVATION_CAPABILITY_BYTES],
    domain: &[u8],
    chunks: &[&[u8]],
    tag: &[u8],
) -> bool {
    let mut mac = HmacSha256::new_from_slice(capability)
        .expect("HMAC-SHA-256 accepts the fixed-length activation capability");
    mac.update(domain);
    for chunk in chunks {
        mac.update(chunk);
    }
    mac.verify_slice(tag).is_ok()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DescriptorLockState {
    Held,
    Contended,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DescriptorReadState {
    Missing,
    Valid,
    Malformed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OwnerLiveness {
    Live,
    Missing,
    Reused,
    Indeterminate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DescriptorStartupDecision {
    CreateNew,
    RemoveStaleThenCreate,
    ForwardExisting,
    RetryReadOnly,
    Block,
}

/// This state machine deliberately never allows a reader to delete state.
/// Deletion is possible only while its protected lease is held *and* the
/// descriptor parses and proves that the recorded owner has exited or its PID
/// has been reused. Any other uncertainty is a fail-closed startup error.
pub(crate) fn decide_descriptor_startup(
    lock: DescriptorLockState,
    descriptor: DescriptorReadState,
    owner: Option<OwnerLiveness>,
) -> DescriptorStartupDecision {
    match (lock, descriptor, owner) {
        (DescriptorLockState::Held, DescriptorReadState::Missing, _) => {
            DescriptorStartupDecision::CreateNew
        }
        (
            DescriptorLockState::Held,
            DescriptorReadState::Valid,
            Some(OwnerLiveness::Missing | OwnerLiveness::Reused),
        ) => DescriptorStartupDecision::RemoveStaleThenCreate,
        (DescriptorLockState::Contended, DescriptorReadState::Valid, _) => {
            DescriptorStartupDecision::ForwardExisting
        }
        (
            DescriptorLockState::Contended,
            DescriptorReadState::Missing | DescriptorReadState::Malformed,
            _,
        ) => DescriptorStartupDecision::RetryReadOnly,
        _ => DescriptorStartupDecision::Block,
    }
}

/// Canonical SDDL allow-list for state and lock handles after creation. The
/// only accepted owners are the built-in Administrators group or LocalSystem;
/// a current-user owner is deliberately rejected regardless of its DACL.
pub(crate) fn is_expected_static_object_sddl(sddl: &str) -> bool {
    matches!(
        sddl,
        "O:BAD:P(A;;FA;;;SY)(A;;FA;;;BA)"
            | "O:BAD:P(A;;FA;;;BA)(A;;FA;;;SY)"
            | "O:SYD:P(A;;FA;;;SY)(A;;FA;;;BA)"
            | "O:SYD:P(A;;FA;;;BA)(A;;FA;;;SY)"
    )
}

/// Canonical SDDL allow-list for the static WiX runtime root. It must be a
/// protected SYSTEM/Administrators-only directory before any descriptor is
/// read. `AI` is accepted because MSI can preserve the auto-inherited bit
/// while retaining the same protected ACE set.
pub(crate) fn is_expected_runtime_root_sddl(sddl: &str) -> bool {
    matches!(
        sddl,
        "O:SYD:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)"
            | "O:SYD:PAI(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)"
            | "O:SYD:P(A;OICI;FA;;;BA)(A;OICI;FA;;;SY)"
            | "O:SYD:PAI(A;OICI;FA;;;BA)(A;OICI;FA;;;SY)"
            | "O:BAD:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)"
            | "O:BAD:PAI(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)"
            | "O:BAD:P(A;OICI;FA;;;BA)(A;OICI;FA;;;SY)"
            | "O:BAD:PAI(A;OICI;FA;;;BA)(A;OICI;FA;;;SY)"
    )
}

/// The dynamic pipe still has a deterministic security descriptor shape: its
/// owner remains BA/SY, administrators retain object-management access, and
/// the exact user SID receives only generic read/write data access. A generic
/// all-access user ACE would reintroduce WRITE_DAC through a leaked endpoint.
pub(crate) fn is_expected_pipe_sddl(sddl: &str, user_sid: &str) -> bool {
    let administrators = format!("O:BAD:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GRGW;;;{user_sid})");
    let system = format!("O:SYD:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GRGW;;;{user_sid})");
    sddl == administrators || sddl == system
}

/// Builds the opaque per-user/per-session identity used only in protected
/// ProgramData state and lease filenames. The random v2 pipe name is derived
/// from descriptor entropy instead, and the raw SID is never exposed.
pub(crate) fn business_instance_key(user_sid: &str, session_id: u32) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"fyagent/windows-business-instance/v1\0");
    hasher.update(user_sid.as_bytes());
    hasher.update(b"\0");
    hasher.update(session_id.to_le_bytes());
    let digest = hasher.finalize();
    digest[..16]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(target_os = "windows")]
mod native;

/// Executes the Windows-only, pre-Tauri guard. On every other target this is
/// deliberately a no-op so cross-platform test binaries never inspect a host
/// token or create OS resources.
pub fn early_windows_startup_gate() -> WindowsStartupDisposition {
    #[cfg(target_os = "windows")]
    {
        native::early_windows_startup_gate()
    }

    #[cfg(not(target_os = "windows"))]
    {
        WindowsStartupDisposition::Continue
    }
}

pub fn runtime_privilege_status() -> RuntimePrivilegeStatus {
    #[cfg(target_os = "windows")]
    {
        native::runtime_privilege_status()
    }

    #[cfg(not(target_os = "windows"))]
    {
        RuntimePrivilegeStatus::unsupported()
    }
}

#[cfg(target_os = "windows")]
pub(crate) fn install_activation_handler<F>(handler: F) -> Result<(), WindowsStartupErrorCode>
where
    F: Fn(ActivationEnvelope) + Send + Sync + 'static,
{
    native::install_activation_handler(handler)
}

#[cfg(target_os = "windows")]
pub(crate) fn release_instance_guard() {
    native::release_instance_guard();
}

#[cfg(test)]
mod tests {
    use super::{
        business_instance_key, decide_descriptor_startup, decode_activation_frame,
        decode_handshake_frame, decode_instance_state, encode_activation_auth,
        encode_activation_request, encode_activation_stop, encode_client_hello,
        encode_instance_state, encode_server_proof, evaluate_privilege_gate, is_expected_pipe_sddl,
        is_expected_runtime_root_sddl, is_expected_static_object_sddl, verify_activation_auth,
        verify_server_proof, ActivationWireMessage, DescriptorLockState, DescriptorReadState,
        DescriptorStartupDecision, HandshakeMessage, InstanceState, InteractiveUserMatch,
        OwnerLiveness, PrivilegeGateDecision, RuntimePrivilegePlatform, RuntimePrivilegeStatus,
        WindowsStartupErrorCode, ACTIVATION_FRAME_BYTES,
    };

    fn windows_status(
        elevated: bool,
        local_administrator: bool,
        interactive_user_match: InteractiveUserMatch,
    ) -> RuntimePrivilegeStatus {
        RuntimePrivilegeStatus {
            platform: RuntimePrivilegePlatform::Windows,
            supported: true,
            elevated,
            local_administrator,
            interactive_user_match,
        }
    }

    #[test]
    fn activation_protocol_round_trips_bounded_raw_arguments() {
        let frame = encode_activation_request(vec![
            "FyAgent.exe".to_string(),
            "fyagent://v1/import?resource=provider&name=Example".to_string(),
        ])
        .expect("bounded activation is encoded");

        assert_eq!(frame.as_bytes().len(), ACTIVATION_FRAME_BYTES);
        assert_eq!(
            decode_activation_frame(frame.as_bytes()),
            Ok(ActivationWireMessage::Request(super::ActivationEnvelope {
                args: vec![
                    "FyAgent.exe".to_string(),
                    "fyagent://v1/import?resource=provider&name=Example".to_string(),
                ],
            }))
        );
    }

    #[test]
    fn activation_protocol_rejects_tampering_controls_and_trailing_data() {
        let mut frame =
            encode_activation_request(vec!["FyAgent.exe".to_string()]).expect("fixture frame");
        frame.as_mut_bytes()[0] ^= 1;
        assert!(decode_activation_frame(frame.as_bytes()).is_err());

        assert!(encode_activation_request(vec!["bad\nargument".to_string()]).is_err());

        let mut frame =
            encode_activation_request(vec!["FyAgent.exe".to_string()]).expect("fixture frame");
        let last = frame.as_mut_bytes().len() - 1;
        frame.as_mut_bytes()[last] = 1;
        assert!(decode_activation_frame(frame.as_bytes()).is_err());
    }

    #[test]
    fn stop_frame_has_no_payload_or_untrusted_arguments() {
        let frame = encode_activation_stop();
        assert_eq!(
            decode_activation_frame(frame.as_bytes()),
            Ok(ActivationWireMessage::Stop)
        );
    }

    #[test]
    fn release_gate_requires_runtime_proof_but_development_does_not_require_uac() {
        let no_elevation = windows_status(true, true, InteractiveUserMatch::Match);
        assert_eq!(
            evaluate_privilege_gate(
                true,
                RuntimePrivilegeStatus {
                    elevated: false,
                    ..no_elevation
                }
            ),
            PrivilegeGateDecision::Block(WindowsStartupErrorCode::RequiredElevationMissing)
        );
        assert_eq!(
            evaluate_privilege_gate(
                true,
                windows_status(true, true, InteractiveUserMatch::Mismatch)
            ),
            PrivilegeGateDecision::Block(WindowsStartupErrorCode::InteractiveUserMismatch)
        );
        assert_eq!(
            evaluate_privilege_gate(
                true,
                windows_status(true, true, InteractiveUserMatch::Unavailable)
            ),
            PrivilegeGateDecision::Block(WindowsStartupErrorCode::InteractiveUserUnavailable)
        );
        assert_eq!(
            evaluate_privilege_gate(
                true,
                windows_status(true, false, InteractiveUserMatch::Match)
            ),
            PrivilegeGateDecision::Block(
                WindowsStartupErrorCode::RequiredLocalAdministratorMissing
            )
        );
        assert_eq!(
            evaluate_privilege_gate(
                false,
                windows_status(false, false, InteractiveUserMatch::Unavailable)
            ),
            PrivilegeGateDecision::Allow
        );
    }

    #[test]
    fn instance_key_is_stable_and_does_not_expose_the_sid() {
        let sid = "S-1-5-21-1000-1001-1002-1003";
        let first = business_instance_key(sid, 7);
        assert_eq!(first, business_instance_key(sid, 7));
        assert_ne!(first, business_instance_key(sid, 8));
        assert_eq!(first.len(), 32);
        assert!(!first.contains(sid));
    }

    #[test]
    fn renderer_status_serializes_only_safe_runtime_facts() {
        let value = serde_json::to_value(windows_status(true, true, InteractiveUserMatch::Match))
            .expect("safe status serializes");
        assert_eq!(
            value,
            serde_json::json!({
                "platform": "windows",
                "supported": true,
                "elevated": true,
                "localAdministrator": true,
                "interactiveUserMatch": "match",
            })
        );
    }

    fn instance_state() -> InstanceState {
        InstanceState::new(
            4242,
            0x0123_4567_89ab_cdef,
            [0x5a; super::PIPE_NONCE_BYTES],
            [0xa5; super::ACTIVATION_CAPABILITY_BYTES],
        )
        .expect("non-zero fixed descriptor fixture")
    }

    #[test]
    fn protected_descriptor_is_fixed_bounded_and_keeps_endpoint_unpredictable() {
        let state = instance_state();
        let frame = encode_instance_state(&state);
        let decoded = decode_instance_state(frame.as_bytes()).expect("descriptor round trips");

        assert_eq!(decoded.owner_pid(), 4242);
        assert_eq!(decoded.owner_creation_time(), 0x0123_4567_89ab_cdef);
        assert_eq!(
            decoded.pipe_name(),
            r"\\.\pipe\FyAgent.Activation.v2.5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5"
        );

        let mut corrupt = frame.as_bytes().to_vec();
        corrupt[9] = 1;
        assert!(decode_instance_state(&corrupt).is_err());
        corrupt[9] = 0;
        corrupt[95] = 1;
        assert!(decode_instance_state(&corrupt).is_err());
    }

    #[test]
    fn capability_handshake_authenticates_server_before_argv_and_binds_request() {
        let state = instance_state();
        let challenge = [0x3c; super::HANDSHAKE_CHALLENGE_BYTES];
        let hello = encode_client_hello(challenge).expect("non-zero challenge");
        assert!(matches!(
            decode_handshake_frame(hello.as_bytes()),
            Ok(HandshakeMessage::ClientHello(returned_challenge)) if returned_challenge == challenge
        ));

        let proof = encode_server_proof(state.capability(), challenge);
        assert!(verify_server_proof(
            state.capability(),
            challenge,
            proof.as_bytes()
        ));
        assert!(!verify_server_proof(
            &[0x44; super::ACTIVATION_CAPABILITY_BYTES],
            challenge,
            proof.as_bytes()
        ));

        let activation = encode_activation_request(vec!["fyagent://v1/import?name=one".into()])
            .expect("activation fixture");
        let auth = encode_activation_auth(state.capability(), &challenge, &activation);
        assert!(verify_activation_auth(
            state.capability(),
            &challenge,
            &activation,
            auth.as_bytes()
        ));

        let changed = encode_activation_request(vec!["fyagent://v1/import?name=two".into()])
            .expect("changed activation fixture");
        assert!(!verify_activation_auth(
            state.capability(),
            &challenge,
            &changed,
            auth.as_bytes()
        ));
    }

    #[test]
    fn descriptor_lifecycle_never_deletes_unparsed_or_live_state() {
        assert_eq!(
            decide_descriptor_startup(
                DescriptorLockState::Held,
                DescriptorReadState::Missing,
                None,
            ),
            DescriptorStartupDecision::CreateNew
        );
        assert_eq!(
            decide_descriptor_startup(
                DescriptorLockState::Held,
                DescriptorReadState::Valid,
                Some(OwnerLiveness::Missing),
            ),
            DescriptorStartupDecision::RemoveStaleThenCreate
        );
        assert_eq!(
            decide_descriptor_startup(
                DescriptorLockState::Held,
                DescriptorReadState::Valid,
                Some(OwnerLiveness::Reused),
            ),
            DescriptorStartupDecision::RemoveStaleThenCreate
        );
        assert_eq!(
            decide_descriptor_startup(
                DescriptorLockState::Held,
                DescriptorReadState::Malformed,
                None,
            ),
            DescriptorStartupDecision::Block
        );
        assert_eq!(
            decide_descriptor_startup(
                DescriptorLockState::Held,
                DescriptorReadState::Valid,
                Some(OwnerLiveness::Live),
            ),
            DescriptorStartupDecision::Block
        );
        assert_eq!(
            decide_descriptor_startup(
                DescriptorLockState::Held,
                DescriptorReadState::Valid,
                Some(OwnerLiveness::Indeterminate),
            ),
            DescriptorStartupDecision::Block
        );
        assert_eq!(
            decide_descriptor_startup(
                DescriptorLockState::Contended,
                DescriptorReadState::Malformed,
                None,
            ),
            DescriptorStartupDecision::RetryReadOnly
        );
    }

    #[test]
    fn protected_descriptor_sddl_rejects_current_user_owners_and_writeable_acl_drift() {
        assert!(is_expected_static_object_sddl(
            "O:BAD:P(A;;FA;;;SY)(A;;FA;;;BA)"
        ));
        assert!(is_expected_static_object_sddl(
            "O:SYD:P(A;;FA;;;BA)(A;;FA;;;SY)"
        ));
        assert!(!is_expected_static_object_sddl(
            "O:S-1-5-21-1000D:P(A;;FA;;;SY)(A;;FA;;;BA)"
        ));
        assert!(!is_expected_static_object_sddl(
            "O:BAD:P(A;;FA;;;SY)(A;;FA;;;BA)(A;;FA;;;BU)"
        ));
        assert!(is_expected_runtime_root_sddl(
            "O:SYD:PAI(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)"
        ));
        assert!(!is_expected_runtime_root_sddl(
            "O:S-1-5-21-1000D:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)"
        ));
        assert!(is_expected_pipe_sddl(
            "O:BAD:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GRGW;;;S-1-5-21-1000)",
            "S-1-5-21-1000",
        ));
        assert!(!is_expected_pipe_sddl(
            "O:BAD:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GA;;;S-1-5-21-1000)",
            "S-1-5-21-1000",
        ));
    }
}

//! Bounded parsing for the one MSIX manifest the installer trusts.
//!
//! An MSIX is a ZIP container, but this module never extracts it.  It audits
//! every entry name before reading only the exact root `AppxManifest.xml` and
//! rejects encrypted, duplicated, path-like, or oversized inputs.  Presence
//! of `AppxSignature.p7x` and `AppxBlockMap.xml` is only a structural gate;
//! Windows PackageManager remains the system trust authority at deployment.

use std::{
    collections::{BTreeMap, HashSet},
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
};

use quick_xml::{
    events::{BytesStart, Event},
    Reader,
};
use zip::ZipArchive;

use crate::codex_desktop::{
    error::{InstallerError, InstallerErrorCode},
    types::{CpuArchitecture, PlatformVersion},
};

const MAX_MSIX_FILE_BYTES: u64 = 4 * 1024 * 1024 * 1024;
const MAX_ZIP_ENTRY_COUNT: usize = 16_384;
// Only the separately bounded root manifest is decompressed here. Keep the
// aggregate declaration bounded without rejecting valid production payloads.
const MAX_ZIP_UNCOMPRESSED_BYTES: u64 = MAX_MSIX_FILE_BYTES;
const MAX_MANIFEST_BYTES: u64 = 512 * 1024;
const MAX_CENTRAL_DIRECTORY_BYTES: u64 = 32 * 1024 * 1024;
const ROOT_MANIFEST_NAME: &str = "AppxManifest.xml";
const BLOCK_MAP_NAME: &str = "AppxBlockMap.xml";
const SIGNATURE_NAME: &str = "AppxSignature.p7x";
const APPX_MANIFEST_NAMESPACE: &str =
    "http://schemas.microsoft.com/appx/manifest/foundation/windows10";

/// The subset of a trusted MSIX manifest used by the platform adapter.
///
/// The parser deliberately does not retain arbitrary elements, executable
/// paths, or XML text.  The installation and launch decisions need only the
/// exact package identity, architecture, version, minimum OS, and one app ID.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WindowsPackageManifest {
    identity_name: String,
    publisher: String,
    version: PlatformVersion,
    architecture: CpuArchitecture,
    minimum_os_version: PlatformVersion,
    application_id: String,
}

impl WindowsPackageManifest {
    pub(crate) fn identity_name(&self) -> &str {
        &self.identity_name
    }

    pub(crate) fn publisher(&self) -> &str {
        &self.publisher
    }

    pub(crate) fn version(&self) -> &PlatformVersion {
        &self.version
    }

    pub(crate) fn architecture(&self) -> CpuArchitecture {
        self.architecture
    }

    pub(crate) fn minimum_os_version(&self) -> &PlatformVersion {
        &self.minimum_os_version
    }
}

/// Parses just the root MSIX manifest from a downloader-owned local package.
pub(crate) fn parse_msix_manifest(
    package_path: &Path,
) -> Result<WindowsPackageManifest, InstallerError> {
    let metadata = std::fs::metadata(package_path)
        .map_err(|_| package_parse_error("MSIX package could not be inspected"))?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_MSIX_FILE_BYTES {
        return Err(package_parse_error(
            "MSIX package size is outside parser bounds",
        ));
    }

    let mut file = File::open(package_path)
        .map_err(|_| package_parse_error("MSIX package could not be opened"))?;
    // zip 2.x stores parsed entries in a map keyed by name, which means a
    // duplicate central-directory name can be overwritten before `len()` or
    // `by_index()` sees it. Audit the bounded raw central directory first.
    audit_zip_central_directory(&mut file, metadata.len())?;
    file.seek(SeekFrom::Start(0))
        .map_err(|_| package_parse_error("MSIX package could not be rewound"))?;
    let mut archive = ZipArchive::new(file)
        .map_err(|_| package_parse_error("MSIX package is not a ZIP archive"))?;
    let manifest = read_root_manifest(&mut archive)?;
    parse_manifest_xml(&manifest)
}

fn audit_zip_central_directory(file: &mut File, file_size: u64) -> Result<(), InstallerError> {
    const END_OF_CENTRAL_DIRECTORY_SIGNATURE: [u8; 4] = [0x50, 0x4B, 0x05, 0x06];
    const CENTRAL_DIRECTORY_ENTRY_SIGNATURE: [u8; 4] = [0x50, 0x4B, 0x01, 0x02];
    const END_OF_CENTRAL_DIRECTORY_SIZE: usize = 22;
    const MAX_EOCD_SEARCH_BYTES: u64 = 65_535 + END_OF_CENTRAL_DIRECTORY_SIZE as u64;
    const CENTRAL_DIRECTORY_ENTRY_SIZE: usize = 46;

    let tail_length = file_size.min(MAX_EOCD_SEARCH_BYTES);
    if tail_length < END_OF_CENTRAL_DIRECTORY_SIZE as u64 {
        return Err(package_parse_error("MSIX ZIP end record is missing"));
    }
    let tail_offset = file_size
        .checked_sub(tail_length)
        .ok_or_else(|| package_parse_error("MSIX ZIP end record is invalid"))?;
    file.seek(SeekFrom::Start(tail_offset))
        .map_err(|_| package_parse_error("MSIX ZIP end record could not be read"))?;
    let mut tail = vec![0_u8; usize::try_from(tail_length).unwrap_or(0)];
    file.read_exact(&mut tail)
        .map_err(|_| package_parse_error("MSIX ZIP end record could not be read"))?;

    let Some(eocd_offset_in_tail) =
        (0..=tail.len() - END_OF_CENTRAL_DIRECTORY_SIZE)
            .rev()
            .find(|offset| {
                tail[*offset..*offset + 4] == END_OF_CENTRAL_DIRECTORY_SIGNATURE
                    && read_zip_u16(&tail, *offset + 20)
                        .map(|comment_length| {
                            *offset + END_OF_CENTRAL_DIRECTORY_SIZE + usize::from(comment_length)
                                == tail.len()
                        })
                        .unwrap_or(false)
            })
    else {
        return Err(package_parse_error("MSIX ZIP end record is missing"));
    };
    let eocd_file_offset = tail_offset
        .checked_add(u64::try_from(eocd_offset_in_tail).unwrap_or(u64::MAX))
        .ok_or_else(|| package_parse_error("MSIX ZIP end record is invalid"))?;

    let disk_number = read_zip_u16(&tail, eocd_offset_in_tail + 4)
        .ok_or_else(|| package_parse_error("MSIX ZIP end record is truncated"))?;
    let central_directory_disk = read_zip_u16(&tail, eocd_offset_in_tail + 6)
        .ok_or_else(|| package_parse_error("MSIX ZIP end record is truncated"))?;
    let entries_on_disk = read_zip_u16(&tail, eocd_offset_in_tail + 8)
        .ok_or_else(|| package_parse_error("MSIX ZIP end record is truncated"))?;
    let entry_count = read_zip_u16(&tail, eocd_offset_in_tail + 10)
        .ok_or_else(|| package_parse_error("MSIX ZIP end record is truncated"))?;
    let central_directory_size = u64::from(
        read_zip_u32(&tail, eocd_offset_in_tail + 12)
            .ok_or_else(|| package_parse_error("MSIX ZIP end record is truncated"))?,
    );
    let central_directory_offset = u64::from(
        read_zip_u32(&tail, eocd_offset_in_tail + 16)
            .ok_or_else(|| package_parse_error("MSIX ZIP end record is truncated"))?,
    );

    if disk_number != 0 || central_directory_disk != 0 {
        return Err(package_parse_error(
            "multi-disk MSIX archives are unsupported",
        ));
    }

    let uses_zip64 = entries_on_disk == u16::MAX
        || entry_count == u16::MAX
        || central_directory_size == u64::from(u32::MAX)
        || central_directory_offset == u64::from(u32::MAX);
    let (entry_count, central_directory_size, central_directory_offset, end_record_offset) =
        if uses_zip64 {
            let zip64 = read_zip64_end_record(file, eocd_file_offset)?;
            if (entries_on_disk != u16::MAX && u64::from(entries_on_disk) != zip64.entries_on_disk)
                || (entry_count != u16::MAX && u64::from(entry_count) != zip64.entry_count)
                || (central_directory_size != u64::from(u32::MAX)
                    && central_directory_size != zip64.central_directory_size)
                || (central_directory_offset != u64::from(u32::MAX)
                    && central_directory_offset != zip64.central_directory_offset)
            {
                return Err(package_parse_error(
                    "MSIX ZIP64 end records are inconsistent",
                ));
            }
            (
                zip64.entry_count,
                zip64.central_directory_size,
                zip64.central_directory_offset,
                zip64.record_offset,
            )
        } else {
            if entries_on_disk != entry_count {
                return Err(package_parse_error(
                    "multi-disk MSIX archives are unsupported",
                ));
            }
            (
                u64::from(entry_count),
                central_directory_size,
                central_directory_offset,
                eocd_file_offset,
            )
        };

    if entry_count == 0 || entry_count > MAX_ZIP_ENTRY_COUNT as u64 {
        return Err(package_parse_error(
            "MSIX ZIP entry count is outside parser bounds",
        ));
    }
    if central_directory_size == 0 || central_directory_size > MAX_CENTRAL_DIRECTORY_BYTES {
        return Err(package_parse_error(
            "MSIX ZIP central directory is outside parser bounds",
        ));
    }
    let central_directory_end = central_directory_offset
        .checked_add(central_directory_size)
        .ok_or_else(|| package_parse_error("MSIX ZIP central directory overflowed"))?;
    if central_directory_end > file_size
        || if uses_zip64 {
            central_directory_end != end_record_offset
        } else {
            central_directory_end > end_record_offset
        }
    {
        return Err(package_parse_error("MSIX ZIP central directory is invalid"));
    }

    file.seek(SeekFrom::Start(central_directory_offset))
        .map_err(|_| package_parse_error("MSIX ZIP central directory could not be read"))?;
    let mut central_directory = vec![0_u8; usize::try_from(central_directory_size).unwrap_or(0)];
    file.read_exact(&mut central_directory)
        .map_err(|_| package_parse_error("MSIX ZIP central directory could not be read"))?;

    let entry_count = usize::try_from(entry_count)
        .map_err(|_| package_parse_error("MSIX ZIP entry count is outside parser bounds"))?;
    let mut names = HashSet::with_capacity(entry_count);
    let mut cursor = 0_usize;
    for _ in 0..entry_count {
        let fixed_end = cursor
            .checked_add(CENTRAL_DIRECTORY_ENTRY_SIZE)
            .ok_or_else(|| package_parse_error("MSIX ZIP central directory overflowed"))?;
        if fixed_end > central_directory.len()
            || central_directory[cursor..cursor + 4] != CENTRAL_DIRECTORY_ENTRY_SIGNATURE
        {
            return Err(package_parse_error(
                "MSIX ZIP central directory entry is invalid",
            ));
        }
        let flags = read_zip_u16(&central_directory, cursor + 8)
            .ok_or_else(|| package_parse_error("MSIX ZIP central directory entry is truncated"))?;
        let name_length = usize::from(
            read_zip_u16(&central_directory, cursor + 28).ok_or_else(|| {
                package_parse_error("MSIX ZIP central directory entry is truncated")
            })?,
        );
        let extra_length = usize::from(
            read_zip_u16(&central_directory, cursor + 30).ok_or_else(|| {
                package_parse_error("MSIX ZIP central directory entry is truncated")
            })?,
        );
        let comment_length = usize::from(
            read_zip_u16(&central_directory, cursor + 32).ok_or_else(|| {
                package_parse_error("MSIX ZIP central directory entry is truncated")
            })?,
        );
        let disk_start = read_zip_u16(&central_directory, cursor + 34)
            .ok_or_else(|| package_parse_error("MSIX ZIP central directory entry is truncated"))?;
        let local_header_offset = u64::from(
            read_zip_u32(&central_directory, cursor + 42).ok_or_else(|| {
                package_parse_error("MSIX ZIP central directory entry is truncated")
            })?,
        );
        if flags & 0x0001 != 0 || disk_start != 0 || local_header_offset >= central_directory_offset
        {
            return Err(package_parse_error(
                "MSIX ZIP central directory entry is unsafe",
            ));
        }
        let name_start = fixed_end;
        let entry_end = name_start
            .checked_add(name_length)
            .and_then(|offset| offset.checked_add(extra_length))
            .and_then(|offset| offset.checked_add(comment_length))
            .ok_or_else(|| package_parse_error("MSIX ZIP central directory entry overflowed"))?;
        if entry_end > central_directory.len() {
            return Err(package_parse_error(
                "MSIX ZIP central directory entry is truncated",
            ));
        }
        let name = std::str::from_utf8(&central_directory[name_start..name_start + name_length])
            .map_err(|_| package_parse_error("MSIX ZIP entry name is not UTF-8"))?;
        if !is_safe_zip_name(name) || !names.insert(name.to_owned()) {
            return Err(package_parse_error(
                "MSIX ZIP entry name is unsafe or duplicated",
            ));
        }
        cursor = entry_end;
    }
    if cursor != central_directory.len() {
        return Err(package_parse_error(
            "MSIX ZIP central directory has trailing data",
        ));
    }
    Ok(())
}

struct Zip64EndRecord {
    record_offset: u64,
    entries_on_disk: u64,
    entry_count: u64,
    central_directory_size: u64,
    central_directory_offset: u64,
}

fn read_zip64_end_record(
    file: &mut File,
    eocd_file_offset: u64,
) -> Result<Zip64EndRecord, InstallerError> {
    const ZIP64_END_RECORD_SIGNATURE: [u8; 4] = [0x50, 0x4B, 0x06, 0x06];
    const ZIP64_END_RECORD_SIZE: u64 = 56;
    const ZIP64_END_RECORD_BODY_SIZE: u64 = 44;
    const ZIP64_END_LOCATOR_SIGNATURE: [u8; 4] = [0x50, 0x4B, 0x06, 0x07];
    const ZIP64_END_LOCATOR_SIZE: u64 = 20;

    let locator_offset = eocd_file_offset
        .checked_sub(ZIP64_END_LOCATOR_SIZE)
        .ok_or_else(|| package_parse_error("MSIX ZIP64 end locator is missing"))?;
    file.seek(SeekFrom::Start(locator_offset))
        .map_err(|_| package_parse_error("MSIX ZIP64 end locator could not be read"))?;
    let mut locator = [0_u8; ZIP64_END_LOCATOR_SIZE as usize];
    file.read_exact(&mut locator)
        .map_err(|_| package_parse_error("MSIX ZIP64 end locator could not be read"))?;
    if locator[..4] != ZIP64_END_LOCATOR_SIGNATURE {
        return Err(package_parse_error("MSIX ZIP64 end locator is missing"));
    }

    let locator_disk = read_zip_u32(&locator, 4)
        .ok_or_else(|| package_parse_error("MSIX ZIP64 end locator is truncated"))?;
    let record_offset = read_zip_u64(&locator, 8)
        .ok_or_else(|| package_parse_error("MSIX ZIP64 end locator is truncated"))?;
    let total_disks = read_zip_u32(&locator, 16)
        .ok_or_else(|| package_parse_error("MSIX ZIP64 end locator is truncated"))?;
    if locator_disk != 0 || total_disks != 1 {
        return Err(package_parse_error(
            "multi-disk MSIX archives are unsupported",
        ));
    }
    if record_offset.checked_add(ZIP64_END_RECORD_SIZE) != Some(locator_offset) {
        return Err(package_parse_error("MSIX ZIP64 end record is misplaced"));
    }

    file.seek(SeekFrom::Start(record_offset))
        .map_err(|_| package_parse_error("MSIX ZIP64 end record could not be read"))?;
    let mut record = [0_u8; ZIP64_END_RECORD_SIZE as usize];
    file.read_exact(&mut record)
        .map_err(|_| package_parse_error("MSIX ZIP64 end record could not be read"))?;
    if record[..4] != ZIP64_END_RECORD_SIGNATURE {
        return Err(package_parse_error("MSIX ZIP64 end record is missing"));
    }
    if read_zip_u64(&record, 4) != Some(ZIP64_END_RECORD_BODY_SIZE) {
        return Err(package_parse_error(
            "MSIX ZIP64 extensible data is unsupported",
        ));
    }

    let disk_number = read_zip_u32(&record, 16)
        .ok_or_else(|| package_parse_error("MSIX ZIP64 end record is truncated"))?;
    let central_directory_disk = read_zip_u32(&record, 20)
        .ok_or_else(|| package_parse_error("MSIX ZIP64 end record is truncated"))?;
    let entries_on_disk = read_zip_u64(&record, 24)
        .ok_or_else(|| package_parse_error("MSIX ZIP64 end record is truncated"))?;
    let entry_count = read_zip_u64(&record, 32)
        .ok_or_else(|| package_parse_error("MSIX ZIP64 end record is truncated"))?;
    let central_directory_size = read_zip_u64(&record, 40)
        .ok_or_else(|| package_parse_error("MSIX ZIP64 end record is truncated"))?;
    let central_directory_offset = read_zip_u64(&record, 48)
        .ok_or_else(|| package_parse_error("MSIX ZIP64 end record is truncated"))?;
    if disk_number != 0 || central_directory_disk != 0 {
        return Err(package_parse_error(
            "multi-disk MSIX archives are unsupported",
        ));
    }
    if entries_on_disk != entry_count {
        return Err(package_parse_error(
            "MSIX ZIP64 end records are inconsistent",
        ));
    }

    Ok(Zip64EndRecord {
        record_offset,
        entries_on_disk,
        entry_count,
        central_directory_size,
        central_directory_offset,
    })
}

fn read_zip_u16(bytes: &[u8], offset: usize) -> Option<u16> {
    let bytes: [u8; 2] = bytes.get(offset..offset.checked_add(2)?)?.try_into().ok()?;
    Some(u16::from_le_bytes(bytes))
}

fn read_zip_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    let bytes: [u8; 4] = bytes.get(offset..offset.checked_add(4)?)?.try_into().ok()?;
    Some(u32::from_le_bytes(bytes))
}

fn read_zip_u64(bytes: &[u8], offset: usize) -> Option<u64> {
    let bytes: [u8; 8] = bytes.get(offset..offset.checked_add(8)?)?.try_into().ok()?;
    Some(u64::from_le_bytes(bytes))
}

fn read_root_manifest<R: Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
) -> Result<Vec<u8>, InstallerError> {
    if archive.is_empty() || archive.len() > MAX_ZIP_ENTRY_COUNT {
        return Err(package_parse_error(
            "MSIX ZIP entry count is outside parser bounds",
        ));
    }

    let mut seen_names = HashSet::with_capacity(archive.len());
    let mut manifest_index = None;
    let mut has_block_map = false;
    let mut has_signature = false;
    let mut uncompressed_total = 0_u64;

    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .map_err(|_| package_parse_error("MSIX ZIP entry could not be read"))?;
        let raw_name = std::str::from_utf8(entry.name_raw())
            .map_err(|_| package_parse_error("MSIX ZIP entry name is not UTF-8"))?;
        if !is_safe_zip_name(raw_name) {
            return Err(package_parse_error("MSIX ZIP entry path is unsafe"));
        }
        if !seen_names.insert(raw_name.to_owned()) {
            return Err(package_parse_error(
                "MSIX ZIP contains a duplicate entry name",
            ));
        }
        if entry.encrypted() {
            return Err(package_parse_error(
                "encrypted MSIX ZIP entries are unsupported",
            ));
        }

        uncompressed_total = checked_zip_uncompressed_total(uncompressed_total, entry.size())?;

        match raw_name {
            ROOT_MANIFEST_NAME => {
                if entry.size() == 0 || entry.size() > MAX_MANIFEST_BYTES {
                    return Err(package_parse_error(
                        "AppxManifest.xml size is outside parser bounds",
                    ));
                }
                manifest_index = Some(index);
            }
            BLOCK_MAP_NAME => has_block_map = true,
            SIGNATURE_NAME => has_signature = true,
            _ => {}
        }
    }

    if !has_block_map {
        return Err(package_parse_error("MSIX AppxBlockMap.xml is missing"));
    }
    if !has_signature {
        return Err(
            InstallerError::new(InstallerErrorCode::PackageSignatureInvalid)
                .with_diagnostic_message("MSIX AppxSignature.p7x is missing"),
        );
    }
    let manifest_index =
        manifest_index.ok_or_else(|| package_parse_error("root AppxManifest.xml is missing"))?;
    let mut manifest_entry = archive
        .by_index(manifest_index)
        .map_err(|_| package_parse_error("root AppxManifest.xml could not be opened"))?;
    let mut manifest = Vec::with_capacity(manifest_entry.size() as usize);
    let mut bounded = (&mut manifest_entry).take(MAX_MANIFEST_BYTES + 1);
    bounded
        .read_to_end(&mut manifest)
        .map_err(|_| package_parse_error("root AppxManifest.xml could not be read"))?;
    if manifest.is_empty() || manifest.len() as u64 > MAX_MANIFEST_BYTES {
        return Err(package_parse_error(
            "root AppxManifest.xml exceeds parser bounds",
        ));
    }
    Ok(manifest)
}

fn checked_zip_uncompressed_total(current: u64, entry_size: u64) -> Result<u64, InstallerError> {
    let total = current
        .checked_add(entry_size)
        .ok_or_else(|| package_parse_error("MSIX ZIP uncompressed size overflowed"))?;
    if total > MAX_ZIP_UNCOMPRESSED_BYTES {
        return Err(package_parse_error(
            "MSIX ZIP uncompressed size exceeds parser bounds",
        ));
    }
    Ok(total)
}

fn is_safe_zip_name(name: &str) -> bool {
    let name = name.strip_suffix('/').unwrap_or(name);
    !name.is_empty()
        && !name.starts_with('/')
        && !name.contains('\\')
        && !name.contains('\0')
        && name
            .split('/')
            .all(|component| !component.is_empty() && component != "." && component != "..")
}

fn parse_manifest_xml(bytes: &[u8]) -> Result<WindowsPackageManifest, InstallerError> {
    let xml = std::str::from_utf8(bytes)
        .map_err(|_| package_parse_error("AppxManifest.xml must be UTF-8"))?;
    let xml = xml.strip_prefix('\u{feff}').unwrap_or(xml);
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut stack = Vec::<Vec<u8>>::new();
    let mut root_seen = false;
    let mut accumulator = ManifestAccumulator::default();

    loop {
        match reader.read_event() {
            Ok(Event::Start(element)) => {
                process_start(&element, &stack, &mut root_seen, &mut accumulator)?;
                stack.push(element.name().as_ref().to_vec());
            }
            Ok(Event::Empty(element)) => {
                process_start(&element, &stack, &mut root_seen, &mut accumulator)?;
            }
            Ok(Event::End(element)) => {
                let closing_name = element.name();
                let closing = closing_name.as_ref();
                let Some(opening) = stack.pop() else {
                    return Err(package_parse_error(
                        "AppxManifest.xml has an unmatched closing tag",
                    ));
                };
                if opening.as_slice() != closing {
                    return Err(package_parse_error(
                        "AppxManifest.xml element nesting is invalid",
                    ));
                }
            }
            Ok(Event::DocType(_) | Event::GeneralRef(_)) => {
                return Err(package_parse_error(
                    "AppxManifest.xml DTD and entity references are forbidden",
                ));
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(_) => return Err(package_parse_error("AppxManifest.xml is malformed")),
        }
    }

    if !root_seen || !stack.is_empty() {
        return Err(package_parse_error(
            "AppxManifest.xml root element is incomplete",
        ));
    }
    accumulator.finish()
}

#[derive(Default)]
struct ManifestAccumulator {
    identity: Option<ManifestIdentity>,
    target_device_family_minimum: Option<PlatformVersion>,
    application_id: Option<String>,
}

impl ManifestAccumulator {
    fn finish(self) -> Result<WindowsPackageManifest, InstallerError> {
        let identity = self
            .identity
            .ok_or_else(|| package_parse_error("AppxManifest.xml Identity is missing"))?;
        let minimum_os_version = self.target_device_family_minimum.ok_or_else(|| {
            package_parse_error("Windows.Desktop TargetDeviceFamily MinVersion is missing")
        })?;
        let application_id = self
            .application_id
            .ok_or_else(|| package_parse_error("exactly one launchable Application is required"))?;
        Ok(WindowsPackageManifest {
            identity_name: identity.name,
            publisher: identity.publisher,
            version: identity.version,
            architecture: identity.architecture,
            minimum_os_version,
            application_id,
        })
    }
}

struct ManifestIdentity {
    name: String,
    publisher: String,
    version: PlatformVersion,
    architecture: CpuArchitecture,
}

fn process_start(
    element: &BytesStart<'_>,
    stack: &[Vec<u8>],
    root_seen: &mut bool,
    accumulator: &mut ManifestAccumulator,
) -> Result<(), InstallerError> {
    let element_name = element.name();
    let name = element_name.as_ref();
    if stack.is_empty() {
        if *root_seen || name != b"Package" {
            return Err(package_parse_error(
                "AppxManifest.xml must have one Package root",
            ));
        }
        validate_package_root_namespace(element)?;
        *root_seen = true;
        return Ok(());
    }

    if stack == [b"Package".to_vec()] && name == b"Identity" {
        if accumulator.identity.is_some() {
            return Err(package_parse_error(
                "AppxManifest.xml has duplicate Identity elements",
            ));
        }
        accumulator.identity = Some(parse_identity(element)?);
    } else if stack == [b"Package".to_vec(), b"Dependencies".to_vec()]
        && name == b"TargetDeviceFamily"
    {
        if accumulator.target_device_family_minimum.is_some() {
            return Err(package_parse_error(
                "AppxManifest.xml has duplicate Windows.Desktop TargetDeviceFamily elements",
            ));
        }
        accumulator.target_device_family_minimum = Some(parse_target_device_family(element)?);
    } else if stack == [b"Package".to_vec(), b"Applications".to_vec()] && name == b"Application" {
        if accumulator.application_id.is_some() {
            return Err(package_parse_error(
                "AppxManifest.xml has multiple launchable Application elements",
            ));
        }
        accumulator.application_id = Some(parse_application_id(element)?);
    }
    Ok(())
}

fn validate_package_root_namespace(element: &BytesStart<'_>) -> Result<(), InstallerError> {
    let mut default_namespace = None;
    for attribute in element.attributes() {
        let attribute =
            attribute.map_err(|_| package_parse_error("AppxManifest.xml attribute is invalid"))?;
        if attribute.key.as_ref() != b"xmlns" {
            continue;
        }
        let value = attribute
            .unescape_value()
            .map_err(|_| package_parse_error("AppxManifest.xml root namespace is invalid"))?
            .into_owned();
        if default_namespace.replace(value).is_some() {
            return Err(package_parse_error(
                "AppxManifest.xml has duplicate default namespaces",
            ));
        }
    }
    if default_namespace.as_deref() != Some(APPX_MANIFEST_NAMESPACE) {
        return Err(package_parse_error(
            "AppxManifest.xml root namespace is not the MSIX foundation namespace",
        ));
    }
    Ok(())
}

fn parse_identity(element: &BytesStart<'_>) -> Result<ManifestIdentity, InstallerError> {
    let attributes = attributes(element)?;
    let name = required_attribute(&attributes, "Name", 256)?;
    let publisher = required_attribute(&attributes, "Publisher", 1024)?;
    let version =
        PlatformVersion::parse_windows_msix(&required_attribute(&attributes, "Version", 64)?)
            .map_err(|_| package_parse_error("MSIX Identity Version is invalid"))?;
    let architecture = match required_attribute(&attributes, "ProcessorArchitecture", 32)?.as_str()
    {
        "x64" => CpuArchitecture::X86_64,
        "arm64" => CpuArchitecture::Aarch64,
        _ => {
            return Err(
                InstallerError::new(InstallerErrorCode::PackageArchitectureMismatch)
                    .with_diagnostic_message("MSIX ProcessorArchitecture is not supported by V1"),
            )
        }
    };
    Ok(ManifestIdentity {
        name,
        publisher,
        version,
        architecture,
    })
}

fn parse_target_device_family(element: &BytesStart<'_>) -> Result<PlatformVersion, InstallerError> {
    let attributes = attributes(element)?;
    if required_attribute(&attributes, "Name", 128)? != "Windows.Desktop" {
        return Err(package_parse_error(
            "MSIX TargetDeviceFamily must be Windows.Desktop",
        ));
    }
    PlatformVersion::parse_windows_msix(&required_attribute(&attributes, "MinVersion", 64)?)
        .map_err(|_| package_parse_error("MSIX TargetDeviceFamily MinVersion is invalid"))
}

fn parse_application_id(element: &BytesStart<'_>) -> Result<String, InstallerError> {
    let attributes = attributes(element)?;
    let id = required_attribute(&attributes, "Id", 256)?;
    if !id
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-'))
    {
        return Err(package_parse_error("MSIX Application Id is invalid"));
    }
    Ok(id)
}

fn attributes(element: &BytesStart<'_>) -> Result<BTreeMap<String, String>, InstallerError> {
    let mut result = BTreeMap::new();
    for attribute in element.attributes() {
        let attribute =
            attribute.map_err(|_| package_parse_error("AppxManifest.xml attribute is invalid"))?;
        let key = std::str::from_utf8(attribute.key.as_ref())
            .map_err(|_| package_parse_error("AppxManifest.xml attribute name is not UTF-8"))?;
        if key == "xmlns" || key.starts_with("xmlns:") {
            continue;
        }
        if key.contains(':') {
            return Err(package_parse_error(
                "AppxManifest.xml trusted attribute names must not be namespaced",
            ));
        }
        let value = attribute
            .unescape_value()
            .map_err(|_| package_parse_error("AppxManifest.xml attribute value is invalid"))?
            .into_owned();
        if result.insert(key.to_owned(), value).is_some() {
            return Err(package_parse_error(
                "AppxManifest.xml has duplicate attributes",
            ));
        }
    }
    Ok(result)
}

fn required_attribute(
    attributes: &BTreeMap<String, String>,
    key: &str,
    maximum_length: usize,
) -> Result<String, InstallerError> {
    let value = attributes
        .get(key)
        .ok_or_else(|| package_parse_error("AppxManifest.xml is missing a required attribute"))?;
    if value.is_empty()
        || value.len() > maximum_length
        || value.bytes().any(|byte| byte.is_ascii_control())
    {
        return Err(package_parse_error(
            "AppxManifest.xml attribute value is outside parser bounds",
        ));
    }
    Ok(value.to_owned())
}

fn package_parse_error(message: &'static str) -> InstallerError {
    InstallerError::new(InstallerErrorCode::PackageParseFailed).with_diagnostic_message(message)
}

#[cfg(test)]
pub(super) fn manifest_for_test(
    identity_name: &str,
    publisher: &str,
    architecture: CpuArchitecture,
    version: &str,
    minimum_os_version: &str,
    application_id: &str,
) -> WindowsPackageManifest {
    WindowsPackageManifest {
        identity_name: identity_name.to_owned(),
        publisher: publisher.to_owned(),
        version: PlatformVersion::parse_windows_msix(version).unwrap(),
        architecture,
        minimum_os_version: PlatformVersion::parse_windows_msix(minimum_os_version).unwrap(),
        application_id: application_id.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use std::{fs::File, io::Write, path::Path};

    use tempfile::tempdir;
    use zip::{write::SimpleFileOptions, ZipWriter};

    use super::*;

    const PUBLISHER: &str = "CN=fixture publisher";
    type Zip64BoundaryMutation = (&'static str, &'static str, Box<dyn Fn(&mut [u8])>);

    fn manifest(architecture: &str, identity: &str, publisher: &str, inner: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="utf-8"?>
<Package xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10">
  <Identity Name="{identity}" Publisher="{publisher}" Version="1.2.3.4" ProcessorArchitecture="{architecture}" />
  <Dependencies><TargetDeviceFamily Name="Windows.Desktop" MinVersion="10.0.19041.0" /></Dependencies>
  <Applications>{inner}</Applications>
</Package>"#,
        )
    }

    fn app() -> &'static str {
        r#"<Application Id="CodexApp" Executable="Codex.exe" EntryPoint="Windows.FullTrustApplication" />"#
    }

    fn write_msix(path: &Path, entries: &[(&str, String)]) {
        let file = File::create(path).unwrap();
        let mut archive = ZipWriter::new(file);
        for (name, contents) in entries {
            archive
                .start_file(*name, SimpleFileOptions::default())
                .unwrap();
            archive.write_all(contents.as_bytes()).unwrap();
        }
        archive.finish().unwrap();
    }

    fn write_zip64_msix(path: &Path, entries: &[(&str, String)]) {
        write_msix(path, entries);
        let mut archive = std::fs::read(path).unwrap();
        let eocd_offset = archive.len().checked_sub(22).unwrap();
        assert_eq!(read_zip_u32(&archive, eocd_offset), Some(0x0605_4B50));
        assert_eq!(read_zip_u16(&archive, eocd_offset + 20), Some(0));

        let entry_count = u64::from(read_zip_u16(&archive, eocd_offset + 10).unwrap());
        let central_directory_size = u64::from(read_zip_u32(&archive, eocd_offset + 12).unwrap());
        let central_directory_offset = u64::from(read_zip_u32(&archive, eocd_offset + 16).unwrap());
        archive.truncate(eocd_offset);

        let zip64_record_offset = u64::try_from(archive.len()).unwrap();
        write_u32(&mut archive, 0x0606_4B50);
        write_u64(&mut archive, 44);
        write_u16(&mut archive, 45);
        write_u16(&mut archive, 45);
        write_u32(&mut archive, 0);
        write_u32(&mut archive, 0);
        write_u64(&mut archive, entry_count);
        write_u64(&mut archive, entry_count);
        write_u64(&mut archive, central_directory_size);
        write_u64(&mut archive, central_directory_offset);

        write_u32(&mut archive, 0x0706_4B50);
        write_u32(&mut archive, 0);
        write_u64(&mut archive, zip64_record_offset);
        write_u32(&mut archive, 1);

        write_u32(&mut archive, 0x0605_4B50);
        write_u16(&mut archive, 0);
        write_u16(&mut archive, 0);
        write_u16(&mut archive, u16::MAX);
        write_u16(&mut archive, u16::MAX);
        write_u32(&mut archive, u32::MAX);
        write_u32(&mut archive, u32::MAX);
        write_u16(&mut archive, 0);
        std::fs::write(path, archive).unwrap();
    }

    fn audit_msix(path: &Path) -> Result<(), InstallerError> {
        let metadata = std::fs::metadata(path).unwrap();
        let mut file = File::open(path).unwrap();
        audit_zip_central_directory(&mut file, metadata.len())
    }

    /// `ZipWriter` correctly refuses duplicate filenames, so this fixture
    /// writes a minimal stored ZIP by hand to prove the parser also rejects a
    /// package that arrived with a duplicated root manifest central entry.
    fn write_duplicate_manifest_msix(path: &Path, manifest: &str) {
        let entries: [(&str, &[u8]); 4] = [
            (ROOT_MANIFEST_NAME, manifest.as_bytes()),
            (ROOT_MANIFEST_NAME, manifest.as_bytes()),
            (BLOCK_MAP_NAME, b"fixture-block-map"),
            (SIGNATURE_NAME, b"fixture-signature"),
        ];
        let mut archive = Vec::new();
        let mut central_directory = Vec::new();

        for (name, contents) in entries {
            let name = name.as_bytes();
            let offset = u32::try_from(archive.len()).unwrap();
            let crc = crc32(contents);
            let size = u32::try_from(contents.len()).unwrap();

            write_u32(&mut archive, 0x0403_4B50);
            write_u16(&mut archive, 20);
            write_u16(&mut archive, 0);
            write_u16(&mut archive, 0);
            write_u16(&mut archive, 0);
            write_u16(&mut archive, 0);
            write_u32(&mut archive, crc);
            write_u32(&mut archive, size);
            write_u32(&mut archive, size);
            write_u16(&mut archive, u16::try_from(name.len()).unwrap());
            write_u16(&mut archive, 0);
            archive.extend_from_slice(name);
            archive.extend_from_slice(contents);

            write_u32(&mut central_directory, 0x0201_4B50);
            write_u16(&mut central_directory, 20);
            write_u16(&mut central_directory, 20);
            write_u16(&mut central_directory, 0);
            write_u16(&mut central_directory, 0);
            write_u16(&mut central_directory, 0);
            write_u16(&mut central_directory, 0);
            write_u32(&mut central_directory, crc);
            write_u32(&mut central_directory, size);
            write_u32(&mut central_directory, size);
            write_u16(&mut central_directory, u16::try_from(name.len()).unwrap());
            write_u16(&mut central_directory, 0);
            write_u16(&mut central_directory, 0);
            write_u16(&mut central_directory, 0);
            write_u16(&mut central_directory, 0);
            write_u32(&mut central_directory, 0);
            write_u32(&mut central_directory, offset);
            central_directory.extend_from_slice(name);
        }

        let central_directory_offset = u32::try_from(archive.len()).unwrap();
        let central_directory_size = u32::try_from(central_directory.len()).unwrap();
        archive.extend_from_slice(&central_directory);
        write_u32(&mut archive, 0x0605_4B50);
        write_u16(&mut archive, 0);
        write_u16(&mut archive, 0);
        write_u16(&mut archive, u16::try_from(entries.len()).unwrap());
        write_u16(&mut archive, u16::try_from(entries.len()).unwrap());
        write_u32(&mut archive, central_directory_size);
        write_u32(&mut archive, central_directory_offset);
        write_u16(&mut archive, 0);
        std::fs::write(path, archive).unwrap();
    }

    fn write_u16(output: &mut Vec<u8>, value: u16) {
        output.extend_from_slice(&value.to_le_bytes());
    }

    fn write_u32(output: &mut Vec<u8>, value: u32) {
        output.extend_from_slice(&value.to_le_bytes());
    }

    fn write_u64(output: &mut Vec<u8>, value: u64) {
        output.extend_from_slice(&value.to_le_bytes());
    }

    fn replace_u16(output: &mut [u8], offset: usize, value: u16) {
        output[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }

    fn replace_u32(output: &mut [u8], offset: usize, value: u32) {
        output[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn replace_u64(output: &mut [u8], offset: usize, value: u64) {
        output[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }

    fn crc32(contents: &[u8]) -> u32 {
        let mut crc = 0xFFFF_FFFF_u32;
        for byte in contents {
            crc ^= u32::from(*byte);
            for _ in 0..8 {
                let polynomial = if crc & 1 == 0 { 0 } else { 0xEDB8_8320 };
                crc = (crc >> 1) ^ polynomial;
            }
        }
        !crc
    }

    fn valid_entries(manifest: String) -> Vec<(&'static str, String)> {
        vec![
            (ROOT_MANIFEST_NAME, manifest),
            (BLOCK_MAP_NAME, "fixture-block-map".to_owned()),
            (SIGNATURE_NAME, "fixture-signature".to_owned()),
        ]
    }

    #[test]
    fn parses_stable_x64_and_arm64_manifest_fixtures() {
        let directory = tempdir().unwrap();
        for (architecture, expected) in [
            ("x64", CpuArchitecture::X86_64),
            ("arm64", CpuArchitecture::Aarch64),
        ] {
            let package = directory.path().join(format!("{architecture}.msix"));
            write_msix(
                &package,
                &valid_entries(manifest(architecture, "OpenAI.Codex", PUBLISHER, app())),
            );
            let parsed = parse_msix_manifest(&package).unwrap();
            assert_eq!(parsed.identity_name(), "OpenAI.Codex");
            assert_eq!(parsed.publisher(), PUBLISHER);
            assert_eq!(parsed.architecture(), expected);
            assert_eq!(parsed.application_id, "CodexApp");
            assert_eq!(
                parsed.minimum_os_version(),
                &PlatformVersion::parse_windows_msix("10.0.19041.0").unwrap()
            );
        }
    }

    #[test]
    fn parses_single_disk_zip64_msix() {
        let directory = tempdir().unwrap();
        let package = directory.path().join("single-disk-zip64.msix");
        write_zip64_msix(
            &package,
            &valid_entries(manifest("x64", "OpenAI.Codex", PUBLISHER, app())),
        );

        let parsed = parse_msix_manifest(&package).unwrap();
        assert_eq!(parsed.identity_name(), "OpenAI.Codex");
        assert_eq!(parsed.architecture(), CpuArchitecture::X86_64);
    }

    #[test]
    fn bounds_declared_uncompressed_total_without_rejecting_production_scale() {
        const PRODUCTION_SCALE_TOTAL: u64 = 1_948_467_324;

        assert_eq!(
            checked_zip_uncompressed_total(0, PRODUCTION_SCALE_TOTAL).unwrap(),
            PRODUCTION_SCALE_TOTAL
        );

        let oversized = checked_zip_uncompressed_total(MAX_ZIP_UNCOMPRESSED_BYTES, 1).unwrap_err();
        assert_eq!(
            oversized.to_dto().details.redacted_message.as_deref(),
            Some("MSIX ZIP uncompressed size exceeds parser bounds")
        );

        let overflowed = checked_zip_uncompressed_total(u64::MAX, 1).unwrap_err();
        assert_eq!(
            overflowed.to_dto().details.redacted_message.as_deref(),
            Some("MSIX ZIP uncompressed size overflowed")
        );
    }

    #[test]
    fn rejects_malformed_or_unsafe_zip64_boundaries() {
        let directory = tempdir().unwrap();
        let valid_package = directory.path().join("valid-zip64.msix");
        write_zip64_msix(
            &valid_package,
            &valid_entries(manifest("x64", "OpenAI.Codex", PUBLISHER, app())),
        );
        let valid = std::fs::read(valid_package).unwrap();
        let eocd_offset = valid.len() - 22;
        let locator_offset = eocd_offset - 20;
        let record_offset = locator_offset - 56;
        let central_directory_size = read_zip_u64(&valid, record_offset + 40).unwrap();
        let central_directory_offset =
            usize::try_from(read_zip_u64(&valid, record_offset + 48).unwrap()).unwrap();

        let mutations: [Zip64BoundaryMutation; 11] = [
            (
                "missing-locator",
                "MSIX ZIP64 end locator is missing",
                Box::new(move |bytes| replace_u32(bytes, locator_offset, 0)),
            ),
            (
                "misplaced-record",
                "MSIX ZIP64 end record is misplaced",
                Box::new(move |bytes| {
                    replace_u64(bytes, locator_offset + 8, record_offset as u64 + 1)
                }),
            ),
            (
                "missing-record",
                "MSIX ZIP64 end record is missing",
                Box::new(move |bytes| replace_u32(bytes, record_offset, 0)),
            ),
            (
                "extensible-record",
                "MSIX ZIP64 extensible data is unsupported",
                Box::new(move |bytes| replace_u64(bytes, record_offset + 4, 45)),
            ),
            (
                "multiple-disks",
                "multi-disk MSIX archives are unsupported",
                Box::new(move |bytes| replace_u32(bytes, locator_offset + 16, 2)),
            ),
            (
                "entry-on-another-disk",
                "MSIX ZIP central directory entry is unsafe",
                Box::new(move |bytes| replace_u16(bytes, central_directory_offset + 34, 1)),
            ),
            (
                "entry-count-mismatch",
                "MSIX ZIP64 end records are inconsistent",
                Box::new(move |bytes| replace_u64(bytes, record_offset + 24, 2)),
            ),
            (
                "classic-record-mismatch",
                "MSIX ZIP64 end records are inconsistent",
                Box::new(move |bytes| {
                    bytes[eocd_offset + 10..eocd_offset + 12].copy_from_slice(&2_u16.to_le_bytes())
                }),
            ),
            (
                "central-directory-too-large",
                "MSIX ZIP central directory is outside parser bounds",
                Box::new(move |bytes| {
                    replace_u64(bytes, record_offset + 40, MAX_CENTRAL_DIRECTORY_BYTES + 1)
                }),
            ),
            (
                "central-directory-out-of-bounds",
                "MSIX ZIP central directory is invalid",
                Box::new(move |bytes| replace_u64(bytes, record_offset + 48, record_offset as u64)),
            ),
            (
                "data-between-directory-and-record",
                "MSIX ZIP central directory is invalid",
                Box::new(move |bytes| {
                    replace_u64(bytes, record_offset + 40, central_directory_size - 1)
                }),
            ),
        ];

        for (name, expected_diagnostic, mutate) in mutations {
            let package = directory.path().join(format!("{name}.msix"));
            let mut archive = valid.clone();
            mutate(&mut archive);
            std::fs::write(&package, archive).unwrap();
            let error = audit_msix(&package).unwrap_err();
            assert_eq!(
                error.code(),
                InstallerErrorCode::PackageParseFailed,
                "{name} should fail closed"
            );
            assert_eq!(
                error.to_dto().details.redacted_message.as_deref(),
                Some(expected_diagnostic),
                "{name} should reach its intended check"
            );
        }
    }

    #[test]
    fn rejects_missing_signature_and_path_variant_before_reading_manifest() {
        let directory = tempdir().unwrap();
        let missing_signature = directory.path().join("missing-signature.msix");
        write_msix(
            &missing_signature,
            &[
                (
                    ROOT_MANIFEST_NAME,
                    manifest("x64", "OpenAI.Codex", PUBLISHER, app()),
                ),
                (BLOCK_MAP_NAME, "fixture-block-map".to_owned()),
            ],
        );
        assert_eq!(
            parse_msix_manifest(&missing_signature).unwrap_err().code(),
            InstallerErrorCode::PackageSignatureInvalid
        );

        let path_variant = directory.path().join("path-variant.msix");
        write_msix(
            &path_variant,
            &[
                (
                    "nested/AppxManifest.xml",
                    manifest("x64", "OpenAI.Codex", PUBLISHER, app()),
                ),
                (BLOCK_MAP_NAME, "fixture-block-map".to_owned()),
                (SIGNATURE_NAME, "fixture-signature".to_owned()),
            ],
        );
        assert_eq!(
            parse_msix_manifest(&path_variant).unwrap_err().code(),
            InstallerErrorCode::PackageParseFailed
        );
    }

    #[test]
    fn rejects_dtd_duplicate_identity_multiple_apps_bad_architecture_and_bad_minimum_os() {
        let directory = tempdir().unwrap();
        let fixtures = [
            (
                "dtd",
                "<!DOCTYPE Package [<!ENTITY bypass 'x'>]>\n".to_owned()
                    + &manifest("x64", "OpenAI.Codex", PUBLISHER, app()),
                InstallerErrorCode::PackageParseFailed,
            ),
            (
                "duplicate-identity",
                format!(
                    r#"<Package><Identity Name="OpenAI.Codex" Publisher="{PUBLISHER}" Version="1.2.3.4" ProcessorArchitecture="x64" /><Identity Name="OpenAI.Codex" Publisher="{PUBLISHER}" Version="1.2.3.4" ProcessorArchitecture="x64" /><Dependencies><TargetDeviceFamily Name="Windows.Desktop" MinVersion="10.0.19041.0" /></Dependencies><Applications>{}</Applications></Package>"#,
                    app()
                ),
                InstallerErrorCode::PackageParseFailed,
            ),
            (
                "multiple-apps",
                manifest(
                    "x64",
                    "OpenAI.Codex",
                    PUBLISHER,
                    &format!("{}{}", app(), app()),
                ),
                InstallerErrorCode::PackageParseFailed,
            ),
            (
                "bad-architecture",
                manifest("x86", "OpenAI.Codex", PUBLISHER, app()),
                InstallerErrorCode::PackageArchitectureMismatch,
            ),
            (
                "bad-minimum-os",
                manifest("x64", "OpenAI.Codex", PUBLISHER, app()).replace(
                    "MinVersion=\"10.0.19041.0\"",
                    "MinVersion=\"not-a-version\"",
                ),
                InstallerErrorCode::PackageParseFailed,
            ),
            (
                "wrong-root-namespace",
                manifest("x64", "OpenAI.Codex", PUBLISHER, app())
                    .replace(APPX_MANIFEST_NAMESPACE, "https://invalid.example.test/appx"),
                InstallerErrorCode::PackageParseFailed,
            ),
        ];

        for (name, xml, expected_error) in fixtures {
            let package = directory.path().join(format!("{name}.msix"));
            write_msix(&package, &valid_entries(xml));
            assert_eq!(
                parse_msix_manifest(&package).unwrap_err().code(),
                expected_error
            );
        }
    }

    #[test]
    fn rejects_duplicate_root_manifest_entry() {
        let directory = tempdir().unwrap();
        let package = directory.path().join("duplicate-manifest.msix");
        let xml = manifest("x64", "OpenAI.Codex", PUBLISHER, app());
        write_duplicate_manifest_msix(&package, &xml);
        assert_eq!(
            parse_msix_manifest(&package).unwrap_err().code(),
            InstallerErrorCode::PackageParseFailed
        );
    }
}

use std::borrow::Cow;
use std::io::ErrorKind;
use std::{io, slice};
use windows::core::{HRESULT, PCWSTR};
use windows::Wdk::Storage::FileSystem::RtlDosPathNameToNtPathName_U_WithStatus;
use windows::Win32::Foundation::{NTSTATUS, STATUS_ACCESS_DENIED, STATUS_ADDRESS_ALREADY_ASSOCIATED, STATUS_ADDRESS_NOT_ASSOCIATED, STATUS_CONNECTION_ABORTED, STATUS_CONNECTION_REFUSED, STATUS_CONNECTION_RESET, STATUS_DIRECTORY_NOT_EMPTY, STATUS_DIRECTORY_NOT_SUPPORTED, STATUS_DISK_FULL, STATUS_END_OF_FILE, STATUS_FILE_TOO_LARGE, STATUS_HOST_UNREACHABLE, STATUS_INTERRUPTED, STATUS_INVALID_DEVICE_REQUEST, STATUS_INVALID_PARAMETER, STATUS_MEDIA_WRITE_PROTECTED, STATUS_NETWORK_NAME_DELETED, STATUS_NETWORK_UNREACHABLE, STATUS_NOT_A_DIRECTORY, STATUS_NOT_FOUND, STATUS_NOT_SAME_DEVICE, STATUS_NOT_SUPPORTED, STATUS_NO_MEMORY, STATUS_OBJECT_NAME_EXISTS, STATUS_OBJECT_PATH_INVALID, STATUS_PIPE_BROKEN, STATUS_POSSIBLE_DEADLOCK, STATUS_QUOTA_EXCEEDED, STATUS_RESOURCE_IN_USE, STATUS_SUCCESS, STATUS_TIMEOUT, STATUS_TOO_MANY_LINKS, STATUS_UNSUCCESSFUL, UNICODE_STRING};
use windows::Win32::System::WindowsProgramming::RtlFreeUnicodeString;

#[derive(Debug, Default)]
struct OwnedUnicodeString(pub UNICODE_STRING);

impl Drop for OwnedUnicodeString {
    fn drop(&mut self) {
        unsafe { RtlFreeUnicodeString(&raw mut self.0) }
    }
}

/// Attempts to normalize a wide string to an NT object manager path.
///
/// # Returns
/// `None` if the path cannot be converted due to `RtlDosPathNameToNtPathName_U_WithStatus` failing.
///
/// # Panics
/// Panics if the path passed isn't null terminated.
pub fn normalize_path(path: &[u16]) -> windows::core::Result<Vec<u16>> {
    let mut output_path = OwnedUnicodeString::default();
    assert!(path.ends_with(&[0u16]), "path passed to normalize_path is not null terminated");

    let status = unsafe {
        RtlDosPathNameToNtPathName_U_WithStatus(
            PCWSTR::from_raw(path.as_ptr()),
            &raw mut output_path.0,
            None,
            None,
        )
    };

    if status != STATUS_SUCCESS {
        return Err(windows::core::Error::from_hresult(HRESULT::from_nt(status.0)));
    }

    let copied = unsafe {
        let s = slice::from_raw_parts(output_path.0.Buffer.0, output_path.0.Length as usize / 2);
        s.to_vec()
    };

    Ok(copied)
}

/// Attempts to normalize a file path stored as a `UNICODE_STRING` to an NT object manager path.
///
/// # Returns
/// `None` if the path cannot be converted (because it contains embedded NULLs or because
/// `RtlDosPathNameToNtPathName_U_WithStatus` failed).
///
/// # Safety
/// Requires the `UNICODE_STRING` length and pointer fields to be accurate.
pub unsafe fn normalize_unicode_string_path(input_path: &UNICODE_STRING) -> Option<Vec<u16>> {
    let path = unsafe {
        slice::from_raw_parts(input_path.Buffer.0, input_path.MaximumLength as usize / 2)
    };

    let used_path = &path[..input_path.Length as usize / 2];

    if used_path.contains(&0u16) {
        return None;
    }

    let buffer: Cow<'_, [u16]> = if path.len() > used_path.len() && path[used_path.len()] == 0u16 {
        // string is already null-terminated
        Cow::Borrowed(&path[..used_path.len() + 1])
    } else {
        // allocate a new buffer
        let mut v = Vec::with_capacity(used_path.len() + 1);
        v.extend_from_slice(&used_path);
        v.push(0);
        Cow::Owned(v)
    };

    normalize_path(&buffer).ok()
}

/// Attempts to convert an [`io::Error`] to an appropriate [`NTSTATUS`].
///
/// # Panics
/// Panics if supplied an error of kind [`WouldBlock`].
///
/// [`io::Error`]: io::Error
/// [`WouldBlock`]: ErrorKind::WouldBlock
pub fn io_error_to_status(error: &io::Error) -> NTSTATUS {
    match error.kind() {
        ErrorKind::NotFound => STATUS_NOT_FOUND,
        ErrorKind::PermissionDenied => STATUS_ACCESS_DENIED,
        ErrorKind::ConnectionRefused => STATUS_CONNECTION_REFUSED,
        ErrorKind::ConnectionReset => STATUS_CONNECTION_RESET,
        ErrorKind::HostUnreachable => STATUS_HOST_UNREACHABLE,
        ErrorKind::NetworkUnreachable => STATUS_NETWORK_UNREACHABLE,
        ErrorKind::ConnectionAborted => STATUS_CONNECTION_ABORTED,
        ErrorKind::NotConnected => STATUS_UNSUCCESSFUL,
        ErrorKind::AddrInUse => STATUS_ADDRESS_ALREADY_ASSOCIATED,
        ErrorKind::AddrNotAvailable => STATUS_ADDRESS_NOT_ASSOCIATED,
        ErrorKind::NetworkDown => STATUS_NETWORK_UNREACHABLE,
        ErrorKind::BrokenPipe => STATUS_PIPE_BROKEN,
        ErrorKind::AlreadyExists => STATUS_OBJECT_NAME_EXISTS,
        ErrorKind::WouldBlock => panic!("cannot turn WouldBlock into an NTSTATUS"),
        ErrorKind::NotADirectory => STATUS_NOT_A_DIRECTORY,
        ErrorKind::IsADirectory => STATUS_DIRECTORY_NOT_SUPPORTED,
        ErrorKind::DirectoryNotEmpty => STATUS_DIRECTORY_NOT_EMPTY,
        ErrorKind::ReadOnlyFilesystem => STATUS_MEDIA_WRITE_PROTECTED,
        ErrorKind::StaleNetworkFileHandle => STATUS_NETWORK_NAME_DELETED,
        ErrorKind::InvalidInput => STATUS_INVALID_PARAMETER,
        ErrorKind::InvalidData => STATUS_INVALID_PARAMETER,
        ErrorKind::TimedOut => STATUS_TIMEOUT,
        ErrorKind::WriteZero => STATUS_UNSUCCESSFUL,
        ErrorKind::StorageFull => STATUS_DISK_FULL,
        ErrorKind::NotSeekable => STATUS_INVALID_DEVICE_REQUEST,
        ErrorKind::QuotaExceeded => STATUS_QUOTA_EXCEEDED,
        ErrorKind::FileTooLarge => STATUS_FILE_TOO_LARGE,
        ErrorKind::ResourceBusy => STATUS_RESOURCE_IN_USE,
        ErrorKind::ExecutableFileBusy => STATUS_RESOURCE_IN_USE,
        ErrorKind::Deadlock => STATUS_POSSIBLE_DEADLOCK,
        ErrorKind::CrossesDevices => STATUS_NOT_SAME_DEVICE,
        ErrorKind::TooManyLinks => STATUS_TOO_MANY_LINKS,
        ErrorKind::InvalidFilename => STATUS_OBJECT_PATH_INVALID,
        ErrorKind::ArgumentListTooLong => STATUS_INVALID_PARAMETER,
        ErrorKind::Interrupted => STATUS_INTERRUPTED,
        ErrorKind::Unsupported => STATUS_NOT_SUPPORTED,
        ErrorKind::UnexpectedEof => STATUS_END_OF_FILE,
        ErrorKind::OutOfMemory => STATUS_NO_MEMORY,
        _ => STATUS_UNSUCCESSFUL,
    }
}
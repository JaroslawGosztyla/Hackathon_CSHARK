//! Per-user secrets on disk (refresh tokens, calendar subscription links).
//!
//! * [`protect`] / [`unprotect`]: DPAPI for the current user (`CryptProtectData` / `CryptUnprotectData`, no UI)
//!   with app-specific entropy, so other apps of the same user cannot trivially unprotect the blobs.
//!   On other platforms both are [`PlatformError::Unsupported`]: a secret is never written in plain text.
//! * [`write_sealed`] / [`read_sealed`] / [`delete_sealed`]: encrypted files, written atomically
//!   (`<path>.tmp` + rename).
//! * [`random_bytes`]: cryptographically secure random bytes (OAuth PKCE verifier and state).
//!
//! Secrets and the contents of sealed files are never logged.

use std::ffi::OsString;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use crate::PlatformError;

/// Encrypted storage is available on this platform.
pub const SUPPORTED: bool = cfg!(target_os = "windows");

/// Encrypts `plain` for the current Windows user.
pub fn protect(plain: &[u8]) -> Result<Vec<u8>, PlatformError> {
    dpapi::protect(plain)
}

/// Decrypts a blob made by [`protect`] (same Windows user).
pub fn unprotect(sealed: &[u8]) -> Result<Vec<u8>, PlatformError> {
    dpapi::unprotect(sealed)
}

/// Encrypts `plain` and writes it atomically to `path` (parent directories are created).
pub fn write_sealed(path: &Path, plain: &[u8]) -> Result<(), PlatformError> {
    let sealed = protect(plain)?;
    write_atomic(path, &sealed)
}

/// Reads and decrypts `path`; `Ok(None)` when the file does not exist.
pub fn read_sealed(path: &Path) -> Result<Option<Vec<u8>>, PlatformError> {
    let sealed = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err.into()),
    };
    unprotect(&sealed).map(Some)
}

/// Deletes a sealed file; a missing file is not an error.
pub fn delete_sealed(path: &Path) -> Result<(), PlatformError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err.into()),
    }
}

/// `len` cryptographically secure random bytes.
pub fn random_bytes(len: usize) -> Result<Vec<u8>, PlatformError> {
    let mut buffer = vec![0u8; len];
    rng::fill(&mut buffer)?;
    Ok(buffer)
}

fn temp_path(path: &Path) -> PathBuf {
    let mut name = OsString::from(path.as_os_str());
    name.push(".tmp");
    PathBuf::from(name)
}

/// Writes `bytes` to `<path>.tmp` and renames it over `path`.
pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), PlatformError> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    let tmp = temp_path(path);
    fs::write(&tmp, bytes)?;
    if let Err(err) = fs::rename(&tmp, path) {
        let _ = fs::remove_file(&tmp);
        return Err(err.into());
    }
    Ok(())
}

#[cfg(target_os = "windows")]
mod dpapi {
    use std::ffi::c_void;

    use windows::Win32::Foundation::{HLOCAL, LocalFree};
    use windows::Win32::Security::Cryptography::{
        CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN, CryptProtectData, CryptUnprotectData,
    };
    use windows::core::w;

    use crate::PlatformError;

    /// Extra entropy for every FocusMode secret. The value predates the shared module (the Teams token was the
    /// first secret); it must never change, or stored secrets can no longer be decrypted.
    const ENTROPY: &[u8] = b"FocusMode/teams-token/v1";

    fn blob(data: &[u8]) -> Result<CRYPT_INTEGER_BLOB, PlatformError> {
        let len =
            u32::try_from(data.len()).map_err(|_| PlatformError::Io("secret too large".into()))?;
        Ok(CRYPT_INTEGER_BLOB {
            cbData: len,
            // DPAPI never writes through the input pointers.
            pbData: data.as_ptr().cast_mut(),
        })
    }

    /// Copies and frees a DPAPI output blob.
    fn take(output: CRYPT_INTEGER_BLOB) -> Vec<u8> {
        if output.pbData.is_null() {
            return Vec::new();
        }
        let len = usize::try_from(output.cbData).unwrap_or(0);
        // SAFETY: on success DPAPI returns `cbData` readable bytes at `pbData`, allocated with LocalAlloc.
        let bytes = unsafe { std::slice::from_raw_parts(output.pbData, len) }.to_vec();
        // SAFETY: freed exactly once with the allocator DPAPI documents (LocalFree).
        let _ = unsafe { LocalFree(Some(HLOCAL(output.pbData.cast::<c_void>()))) };
        bytes
    }

    pub(super) fn protect(plain: &[u8]) -> Result<Vec<u8>, PlatformError> {
        let input = blob(plain)?;
        let entropy = blob(ENTROPY)?;
        let mut output = CRYPT_INTEGER_BLOB::default();
        // SAFETY: input/entropy point to live slices for the call; `output` is a valid out blob.
        unsafe {
            CryptProtectData(
                &input,
                w!("FocusMode secret"),
                Some(&entropy as *const CRYPT_INTEGER_BLOB),
                None,
                None,
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        }
        .map_err(|err| PlatformError::Os(format!("CryptProtectData failed: {err}")))?;
        Ok(take(output))
    }

    pub(super) fn unprotect(sealed: &[u8]) -> Result<Vec<u8>, PlatformError> {
        let input = blob(sealed)?;
        let entropy = blob(ENTROPY)?;
        let mut output = CRYPT_INTEGER_BLOB::default();
        // SAFETY: input/entropy point to live slices for the call; `output` is a valid out blob.
        unsafe {
            CryptUnprotectData(
                &input,
                None,
                Some(&entropy as *const CRYPT_INTEGER_BLOB),
                None,
                None,
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        }
        .map_err(|err| PlatformError::Os(format!("CryptUnprotectData failed: {err}")))?;
        Ok(take(output))
    }
}

#[cfg(not(target_os = "windows"))]
mod dpapi {
    use crate::PlatformError;

    pub(super) fn protect(_plain: &[u8]) -> Result<Vec<u8>, PlatformError> {
        Err(PlatformError::Unsupported)
    }

    pub(super) fn unprotect(_sealed: &[u8]) -> Result<Vec<u8>, PlatformError> {
        Err(PlatformError::Unsupported)
    }
}

#[cfg(target_os = "windows")]
mod rng {
    use windows::Win32::Security::Cryptography::{
        BCRYPT_USE_SYSTEM_PREFERRED_RNG, BCryptGenRandom,
    };

    use crate::PlatformError;

    pub(super) fn fill(buffer: &mut [u8]) -> Result<(), PlatformError> {
        if buffer.is_empty() {
            return Ok(());
        }
        if u32::try_from(buffer.len()).is_err() {
            return Err(PlatformError::Os("random buffer too large".into()));
        }
        // SAFETY: no algorithm handle with the system-preferred RNG flag; the buffer is a valid mutable slice.
        unsafe { BCryptGenRandom(None, buffer, BCRYPT_USE_SYSTEM_PREFERRED_RNG) }
            .ok()
            .map_err(|err| PlatformError::Os(format!("BCryptGenRandom failed: {err}")))
    }
}

#[cfg(not(target_os = "windows"))]
mod rng {
    use std::io::Read;

    use crate::PlatformError;

    pub(super) fn fill(buffer: &mut [u8]) -> Result<(), PlatformError> {
        let mut source = std::fs::File::open("/dev/urandom")?;
        source.read_exact(buffer)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_temp_path(name: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!(
            "focusmode-test-{}-{nanos}-{name}",
            std::process::id()
        ))
    }

    #[test]
    fn atomic_write_replaces_file() {
        let path = unique_temp_path("atomic.bin");
        write_atomic(&path, b"one").expect("first write");
        write_atomic(&path, b"two").expect("second write");
        assert_eq!(std::fs::read(&path).expect("read"), b"two");
        assert!(!temp_path(&path).exists(), "temp file renamed away");
        delete_sealed(&path).expect("delete");
        delete_sealed(&path).expect("delete twice");
        assert!(!path.exists());
    }

    #[test]
    fn missing_sealed_file_is_none() {
        let path = unique_temp_path("missing.bin");
        assert!(matches!(read_sealed(&path), Ok(None)));
    }

    #[test]
    fn random_bytes_are_random() {
        let a = random_bytes(32).expect("random");
        let b = random_bytes(32).expect("random");
        assert_eq!(a.len(), 32);
        assert_ne!(a, b);
        assert!(random_bytes(0).expect("empty").is_empty());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn dpapi_round_trip() {
        let sealed = protect(b"secret-value").expect("protect");
        assert!(!String::from_utf8_lossy(&sealed).contains("secret-value"));
        assert_eq!(unprotect(&sealed).expect("unprotect"), b"secret-value");
        assert!(unprotect(b"garbage").is_err());

        let path = unique_temp_path("sealed.bin");
        write_sealed(&path, b"https://calendar.example/secret.ics").expect("write");
        let raw = std::fs::read(&path).expect("raw");
        assert!(!String::from_utf8_lossy(&raw).contains("secret.ics"));
        assert_eq!(
            read_sealed(&path).expect("read").as_deref(),
            Some(&b"https://calendar.example/secret.ics"[..])
        );
        std::fs::write(&path, b"garbage").expect("corrupt");
        assert!(read_sealed(&path).is_err());
        delete_sealed(&path).expect("delete");
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn sealing_is_unsupported_elsewhere() {
        assert!(!SUPPORTED);
        assert!(matches!(protect(b"x"), Err(PlatformError::Unsupported)));
        let path = unique_temp_path("sealed.bin");
        assert!(matches!(
            write_sealed(&path, b"x"),
            Err(PlatformError::Unsupported)
        ));
        assert!(!path.exists());
    }
}

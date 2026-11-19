use sha2::{Digest, Sha256};

use crate::application::ports::credentials::{CredentialError, CredentialPort};

pub(super) struct OsCredentialStore;

pub(super) fn credential_reference() -> String {
    match crate::audit_data_dir() {
        // Smoke credentials cannot accidentally read or replace the installed
        // user's production credential entry.
        Some(path) => format!(
            "audit-{:x}",
            Sha256::digest(path.to_string_lossy().as_bytes())
        ),
        None => "codex-reading-v1".to_owned(),
    }
}

#[cfg(target_os = "macos")]
impl OsCredentialStore {
    fn entry(reference: &str) -> Result<keyring::Entry, CredentialError> {
        keyring::Entry::new("com.agentdesk.desktop", reference)
            .map_err(|_| CredentialError::Unavailable)
    }
}

impl CredentialPort for OsCredentialStore {
    fn store(&self, reference: &str, secret: &str) -> Result<(), CredentialError> {
        #[cfg(target_os = "windows")]
        return windows::store(reference, secret);

        #[cfg(target_os = "macos")]
        return Self::entry(reference)?
            .set_password(secret)
            .map_err(|_| CredentialError::OperationFailed);

        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        {
            let _ = (reference, secret);
            Err(CredentialError::Unavailable)
        }
    }

    fn read(&self, reference: &str) -> Result<Option<String>, CredentialError> {
        #[cfg(target_os = "windows")]
        return windows::read(reference);

        #[cfg(target_os = "macos")]
        return match Self::entry(reference)?.get_password() {
            Ok(secret) => Ok(Some(secret)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(_) => Err(CredentialError::OperationFailed),
        };

        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        {
            let _ = reference;
            Err(CredentialError::Unavailable)
        }
    }

    fn delete(&self, reference: &str) -> Result<(), CredentialError> {
        #[cfg(target_os = "windows")]
        return windows::delete(reference);

        #[cfg(target_os = "macos")]
        return match Self::entry(reference)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err(CredentialError::OperationFailed),
        };

        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        {
            let _ = reference;
            Err(CredentialError::Unavailable)
        }
    }
}

#[cfg(target_os = "windows")]
mod windows {
    use std::ptr::{self, NonNull};
    use windows_sys::Win32::{
        Foundation::{GetLastError, ERROR_NOT_FOUND},
        Security::Credentials::{
            CredDeleteW, CredFree, CredReadW, CredWriteW, CREDENTIALW,
            CRED_MAX_CREDENTIAL_BLOB_SIZE, CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC,
        },
    };

    use super::CredentialError;

    fn target(reference: &str) -> Vec<u16> {
        format!("com.agentdesk.desktop/{reference}")
            .encode_utf16()
            .chain([0])
            .collect()
    }

    pub(super) fn store(reference: &str, secret: &str) -> Result<(), CredentialError> {
        let mut name = target(reference);
        let size = u32::try_from(secret.len()).map_err(|_| CredentialError::OperationFailed)?;
        if size == 0 || size > CRED_MAX_CREDENTIAL_BLOB_SIZE {
            return Err(CredentialError::OperationFailed);
        }
        let entry = CREDENTIALW {
            Type: CRED_TYPE_GENERIC,
            TargetName: name.as_mut_ptr(),
            CredentialBlobSize: size,
            CredentialBlob: secret.as_ptr().cast_mut(),
            // Do not roam the Codex bearer through an enterprise profile.
            Persist: CRED_PERSIST_LOCAL_MACHINE,
            ..Default::default()
        };
        // SAFETY: the initialized record and both pointed-to buffers live
        // for the synchronous call. CredWriteW reads but does not modify them.
        if unsafe { CredWriteW(&entry, 0) } == 0 {
            return Err(CredentialError::OperationFailed);
        }
        Ok(())
    }

    struct OwnedCredential(NonNull<CREDENTIALW>);

    impl Drop for OwnedCredential {
        fn drop(&mut self) {
            // SAFETY: only created from a successful CredReadW allocation.
            unsafe { CredFree(self.0.as_ptr().cast()) };
        }
    }

    pub(super) fn read(reference: &str) -> Result<Option<String>, CredentialError> {
        let name = target(reference);
        let mut result = ptr::null_mut();
        // SAFETY: name is a live, NUL-terminated UTF-16 buffer; result is a
        // valid output pointer for CredReadW.
        if unsafe { CredReadW(name.as_ptr(), CRED_TYPE_GENERIC, 0, &mut result) } == 0 {
            // SAFETY: read directly after the failed call on this thread.
            return if unsafe { GetLastError() } == ERROR_NOT_FOUND {
                Ok(None)
            } else {
                Err(CredentialError::OperationFailed)
            };
        }
        let owned = OwnedCredential(NonNull::new(result).ok_or(CredentialError::OperationFailed)?);
        // SAFETY: the operating system owns the returned record and buffer
        // until the guard invokes CredFree.
        let entry = unsafe { owned.0.as_ref() };
        if entry.CredentialBlob.is_null()
            || entry.CredentialBlobSize == 0
            || entry.CredentialBlobSize > CRED_MAX_CREDENTIAL_BLOB_SIZE
            || entry.Persist != CRED_PERSIST_LOCAL_MACHINE
        {
            return Err(CredentialError::OperationFailed);
        }
        // SAFETY: CredReadW supplied the non-null buffer and its byte size.
        let bytes = unsafe {
            std::slice::from_raw_parts(entry.CredentialBlob, entry.CredentialBlobSize as usize)
        };
        std::str::from_utf8(bytes)
            .map(|value| Some(value.to_owned()))
            .map_err(|_| CredentialError::OperationFailed)
    }

    pub(super) fn delete(reference: &str) -> Result<(), CredentialError> {
        let name = target(reference);
        // SAFETY: name is a live, NUL-terminated UTF-16 buffer.
        if unsafe { CredDeleteW(name.as_ptr(), CRED_TYPE_GENERIC, 0) } != 0
            // SAFETY: read immediately after the failed call on this thread.
            || unsafe { GetLastError() } == ERROR_NOT_FOUND
        {
            Ok(())
        } else {
            Err(CredentialError::OperationFailed)
        }
    }
}

#[cfg(all(test, any(target_os = "windows", target_os = "macos")))]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires an unlocked OS credential store; uses a random synthetic identifier"]
    fn os_credential_roundtrip() {
        let store = OsCredentialStore;
        let reference = format!("integration-test-{}", uuid::Uuid::new_v4().simple());
        let synthetic = uuid::Uuid::new_v4().to_string();
        store
            .store(&reference, &synthetic)
            .expect("write isolated OS credential");
        let read = store.read(&reference);
        let deleted = store.delete(&reference);
        assert!(matches!(read, Ok(Some(value)) if value == synthetic));
        deleted.expect("remove isolated OS credential");
        assert!(matches!(store.read(&reference), Ok(None)));
    }
}

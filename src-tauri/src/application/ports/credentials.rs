use thiserror::Error;

#[derive(Debug, Error)]
pub enum CredentialError {
    #[error("credential is unavailable")]
    Unavailable,
    #[error("credential operation failed")]
    OperationFailed,
}

pub trait CredentialPort: Send + Sync {
    fn store(&self, credential_ref: &str, secret: &str) -> Result<(), CredentialError>;
    fn read(&self, credential_ref: &str) -> Result<Option<String>, CredentialError>;
    fn delete(&self, credential_ref: &str) -> Result<(), CredentialError>;
}

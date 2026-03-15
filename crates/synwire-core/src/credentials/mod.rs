//! Credential management and secret handling.

mod env;
mod secret;
mod static_creds;
/// Credential provider trait definitions.
pub mod traits;

pub use env::EnvCredentialProvider;
pub use secret::SecretValue;
pub use static_creds::StaticCredentialProvider;
pub use traits::CredentialProvider;

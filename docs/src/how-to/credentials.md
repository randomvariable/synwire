# Credentials

Synwire provides a credential management system with secret redaction and multiple provider strategies.

## Credential providers

| Provider | Source | Use case |
|----------|--------|----------|
| `EnvCredentialProvider` | Environment variables | Production deployments |
| `StaticCredentialProvider` | Hardcoded values | Testing only |

## Using environment variables

```rust,ignore
use synwire_core::credentials::EnvCredentialProvider;
use synwire_core::credentials::CredentialProvider;

let provider = EnvCredentialProvider::new("OPENAI_API_KEY");
let secret = provider.get_credential()?;
// secret is a SecretValue that redacts on Display/Debug
```

## SecretValue

API keys are wrapped in `SecretValue` (from the `secrecy` crate) to prevent accidental logging:

```rust,ignore
use synwire_core::credentials::SecretValue;

let secret = SecretValue::new("sk-abc123".into());
// println!("{secret}") prints "[REDACTED]"
```

## Static credentials for testing

```rust,ignore
use synwire_core::credentials::StaticCredentialProvider;
use synwire_core::credentials::CredentialProvider;

let provider = StaticCredentialProvider::new("test-key");
let secret = provider.get_credential()?;
```

## Provider integration

Providers like `ChatOpenAI` accept credentials via their builders:

```rust,ignore
use synwire_llm_openai::ChatOpenAI;

// From environment variable
let model = ChatOpenAI::builder()
    .model("gpt-4o-mini")
    .api_key_env("OPENAI_API_KEY")
    .build()?;
```

## Custom credential provider

Implement `CredentialProvider` for custom sources (vaults, config files, etc.):

```rust,ignore
use synwire_core::credentials::CredentialProvider;
use synwire_core::credentials::SecretValue;
use synwire_core::error::SynwireError;

struct VaultCredentialProvider {
    key_name: String,
}

impl CredentialProvider for VaultCredentialProvider {
    fn get_credential(&self) -> Result<SecretValue, SynwireError> {
        // Fetch from vault...
        Ok(SecretValue::new("fetched-key".into()))
    }
}
```

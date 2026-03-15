# Retry and Fallback

Synwire provides built-in retry and fallback mechanisms for handling transient errors.

## Retry configuration

Wrap any `RunnableCore` with retry logic:

```rust,ignore
use synwire_core::runnables::{RunnableRetry, RetryConfig};
use synwire_core::error::SynwireErrorKind;

let config = RetryConfig::new()
    .max_retries(3)
    .retry_on(vec![SynwireErrorKind::Model]);  // Only retry model errors

let retryable = RunnableRetry::new(my_runnable, config);
```

## Fallbacks

Chain multiple runnables as fallbacks. If the primary fails, the next is tried:

```rust,ignore
use synwire_core::runnables::with_fallbacks;
use synwire_core::language_models::{FakeChatModel, BaseChatModel};

let primary = FakeChatModel::new(vec!["Primary response".into()]);
let fallback = FakeChatModel::new(vec!["Fallback response".into()]);

// with_fallbacks tries each in order
let resilient = with_fallbacks(vec![
    Box::new(primary),
    Box::new(fallback),
]);
```

## Error kinds for matching

`SynwireErrorKind` categorises errors for retry/fallback decisions:

| Kind | When to retry |
|------|---------------|
| `Model` | Rate limits, timeouts, transient failures |
| `Tool` | Tool invocation failures |
| `Parse` | Output parsing failures (consider with caution) |
| `Embedding` | Embedding API failures |
| `Credential` | Typically not retryable |
| `Serialization` | Not retryable |

## Combining retry and fallback

```rust,ignore
use synwire_core::runnables::{RunnableRetry, RetryConfig, with_fallbacks};
use synwire_core::error::SynwireErrorKind;

// Primary with retry
let primary_with_retry = RunnableRetry::new(
    primary_model,
    RetryConfig::new()
        .max_retries(2)
        .retry_on(vec![SynwireErrorKind::Model]),
);

// Fallback without retry
let resilient = with_fallbacks(vec![
    Box::new(primary_with_retry),
    Box::new(fallback_model),
]);
```

## Callback on retry

Monitor retries via the `CallbackHandler`:

```rust,ignore
impl CallbackHandler for MyCallback {
    fn on_retry<'a>(
        &'a self,
        attempt: u32,
        error: &'a str,
    ) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            eprintln!("Retry attempt {attempt}: {error}");
        })
    }
}
```

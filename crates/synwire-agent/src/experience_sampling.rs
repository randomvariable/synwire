//! Experience pool summary generation via sampling.
//!
//! Post-edit, generates a summary of the diff and affected files for storage
//! in the experience pool. Falls back to listing raw file associations when
//! sampling is unavailable.

use synwire_core::{SamplingProvider, SamplingRequest};

/// Maximum number of diff characters included in the sampling prompt.
///
/// Diffs longer than this are truncated to keep the request within typical
/// context-window budgets.
const MAX_DIFF_CHARS: usize = 4000;

/// Maximum tokens the model may generate for the summary.
const SUMMARY_MAX_TOKENS: u32 = 256;

/// Sampling temperature — low for deterministic, factual summaries.
const SUMMARY_TEMPERATURE: f32 = 0.3;

/// System prompt instructing the model to produce concise edit summaries.
const SYSTEM_PROMPT: &str = "You are a code review assistant. Generate a concise \
    one-paragraph summary of the following code change. Focus on what was changed and why.";

/// Generate a summary of an edit event for the experience pool.
///
/// When a [`SamplingProvider`] is available, builds a structured prompt from
/// the diff and affected file list and requests a natural-language summary
/// via the provider. The diff is truncated to `MAX_DIFF_CHARS` characters
/// to stay within context-window limits.
///
/// Falls back gracefully to listing affected files when sampling is
/// unavailable or when the sampling call fails for any reason.
///
/// # Blocking behaviour
///
/// This function synchronously blocks on the async [`SamplingProvider::sample`]
/// call using [`tokio::task::block_in_place`]. It must therefore be called
/// from within a multi-threaded tokio runtime (the default for
/// `#[tokio::main]`).
///
/// # Examples
///
/// ```no_run
/// use synwire_core::NoopSamplingProvider;
/// use synwire_agent::experience_sampling::summarize_edit;
/// let p = NoopSamplingProvider;
/// let summary = summarize_edit("- old line\n+ new line", &["src/main.rs"], &p);
/// assert!(summary.contains("src/main.rs"));
/// ```
pub fn summarize_edit(
    diff: &str,
    affected_files: &[&str],
    sampling: &dyn SamplingProvider,
) -> String {
    let fallback = || format!("Files: {}", affected_files.join(", "));

    if !sampling.is_available() {
        return fallback();
    }

    // Truncate the diff to stay within context-window budgets.
    let truncated_diff = if diff.len() > MAX_DIFF_CHARS {
        let mut end = MAX_DIFF_CHARS;
        // Avoid splitting a multi-byte UTF-8 sequence.
        while !diff.is_char_boundary(end) {
            end -= 1;
        }
        &diff[..end]
    } else {
        diff
    };

    let file_list = affected_files.join(", ");
    let prompt = format!("Affected files: {file_list}\n\nDiff:\n```\n{truncated_diff}\n```");

    let request = SamplingRequest::new(prompt)
        .with_system(SYSTEM_PROMPT)
        .with_max_tokens(SUMMARY_MAX_TOKENS)
        .with_temperature(SUMMARY_TEMPERATURE);

    // Block synchronously on the async sampling call using `block_in_place`,
    // which moves the current worker thread out of the scheduler so we can
    // call `block_on` without deadlocking.  `block_in_place` panics on
    // current-thread runtimes, so we detect that flavour and fall back.
    let result = match tokio::runtime::Handle::try_current() {
        Ok(handle) => {
            if handle.runtime_flavor() == tokio::runtime::RuntimeFlavor::CurrentThread {
                return fallback();
            }
            tokio::task::block_in_place(|| handle.block_on(sampling.sample(request)))
        }
        Err(_) => return fallback(),
    };

    match result {
        Ok(response) => response.text,
        Err(_) => fallback(),
    }
}

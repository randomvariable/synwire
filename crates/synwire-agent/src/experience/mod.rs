//! Project-local and global experience pool.
//!
//! Records associations between tasks (prompts/descriptions) and the files
//! they modified, enabling experience-guided file localization for future
//! similar tasks.

mod pool;

pub use pool::{
    ExperienceEntry, ExperienceError, ExperiencePool, TieredExperiencePool, record_edit_completion,
};

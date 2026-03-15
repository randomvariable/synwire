//! Channel types for the Pregel engine.
//!
//! Channels manage state flow between graph nodes. Each channel type implements
//! [`BaseChannel`] with different accumulation semantics:
//!
//! | Channel | Semantics |
//! |---------|-----------|
//! | [`LastValue`] | Keeps only the most recent value |
//! | [`Topic`] | Accumulates all values in order |
//! | [`BinaryOperatorAggregate`] | Folds values with a reducer |
//! | [`AnyValue`] | Picks any one value |
//! | [`EphemeralValue`] | Clears after read |
//! | [`NamedBarrierValue`] | Fires when all named triggers arrive |

pub mod any_value;
pub mod binary_operator;
pub mod ephemeral;
pub mod last_value;
pub mod named_barrier;
pub mod topic;
pub mod traits;

pub use any_value::AnyValue;
pub use binary_operator::BinaryOperatorAggregate;
pub use ephemeral::EphemeralValue;
pub use last_value::LastValue;
pub use named_barrier::NamedBarrierValue;
pub use topic::Topic;
pub use traits::BaseChannel;

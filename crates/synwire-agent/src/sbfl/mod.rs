//! Spectrum-Based Fault Localization (SBFL) using Ochiai scoring.
//!
//! Uses DAP coverage data to rank files/functions by fault likelihood.

mod ochiai;

pub use ochiai::{CoverageRecord, SbflRanker, fuse_sbfl_semantic, ochiai};

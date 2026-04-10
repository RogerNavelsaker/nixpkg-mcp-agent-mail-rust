//! Test fixtures for sqlmodel-console integration tests.

#[allow(dead_code)]
pub mod generators;
pub mod golden;
#[allow(dead_code)]
pub mod mock_types;
pub mod sample_data;

pub use golden::*;
pub use sample_data::*;

//! The pipelines of ranim
pub mod debug;
pub mod oit_resolve;
pub mod vitem;

pub use oit_resolve::OITResolvePipeline;
pub use vitem::{VItemColorPipeline, VItemComputePipeline, VItemDepthPipeline};

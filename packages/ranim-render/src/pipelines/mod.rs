//! The pipelines of ranim
pub mod debug;
pub mod merged_vitem;
pub mod oit_resolve;
pub mod vitem;
pub mod vitem_compute;

pub use merged_vitem::{
    MergedVItemColorPipeline, MergedVItemComputePipeline, MergedVItemDepthPipeline,
};
pub use oit_resolve::OITResolvePipeline;
pub use vitem::{VItemColorPipeline, VItemDepthPipeline};
pub use vitem_compute::VItemComputePipeline;

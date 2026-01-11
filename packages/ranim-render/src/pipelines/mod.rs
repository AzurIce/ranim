//! The pipelines of ranim
pub mod clipbox_2d;
pub mod debug;
pub mod map_3d_to_2d;
pub mod oit_resolve;
pub mod vitem;
pub mod vitem2d;

pub use clipbox_2d::ClipBox2dPipeline;
pub use map_3d_to_2d::Map3dTo2dPipeline;
pub use oit_resolve::OITResolvePipeline;
pub use vitem::VItemPipeline;
pub use vitem2d::{VItem2dColorPipeline, VItem2dDepthPipeline};

use std::sync::Arc;

use crate::utils::PipelinesStorage;

pub struct RanimContext {
    pub wgpu_ctx: Arc<WgpuContext>,
    pub pipelines: PipelinesStorage,
}

impl Default for RanimContext {
    fn default() -> Self {
        Self::new()
    }
}

impl RanimContext {
    pub fn new() -> Self {
        let wgpu_ctx = Arc::new(pollster::block_on(WgpuContext::new()));
        let pipelines = PipelinesStorage::default();

        Self {
            wgpu_ctx,
            pipelines,
        }
    }

    pub fn wgpu_ctx(&self) -> Arc<WgpuContext> {
        self.wgpu_ctx.clone()
    }
}

pub struct WgpuContext {
    // pub instance: wgpu::Instance,
    // pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

impl WgpuContext {
    pub async fn new() -> Self {
        let instance = wgpu::Instance::default();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                force_fallback_adapter: true,
                ..Default::default()
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await
            .unwrap();

        Self {
            // instance,
            // adapter,
            device,
            queue,
        }
    }
}

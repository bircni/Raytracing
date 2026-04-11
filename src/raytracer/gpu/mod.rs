//! GPU-accelerated Whitted raytracing (compute shader).

mod pipeline;
mod scene;
mod types;

pub use pipeline::GpuRaytrace;

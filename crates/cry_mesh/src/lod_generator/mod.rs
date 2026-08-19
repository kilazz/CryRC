pub mod auto_generator;
pub mod auto_lod_settings;
pub mod auto_uv;
pub mod lod_builder;
pub mod types;
pub mod visual_change;

pub use auto_generator::AutoGenerator;
pub use auto_lod_settings::AutoLodSettings;
pub use auto_uv::AutoUV;
pub use lod_builder::LODMeshBuilder;
pub use types::{LODGenParams, LODSequenceOutput};
pub use visual_change::VisualChangeCalculator;

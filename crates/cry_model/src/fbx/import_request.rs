use cry_mesh::PhysGeomType;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialInfo {
    #[serde(rename = "name")]
    pub source_name: String,
    #[serde(default = "default_phys")]
    pub physicalize: String,
}

fn default_phys() -> String {
    "none".to_string()
}

impl MaterialInfo {
    pub fn get_physics_type(&self) -> PhysGeomType {
        match self.physicalize.to_ascii_lowercase().as_str() {
            "default" => PhysGeomType::Default,
            "no_collide" => PhysGeomType::NoCollide,
            "obstruct" => PhysGeomType::Obstruct,
            _ => PhysGeomType::None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AnimationSettings {
    #[serde(rename = "startFrame", default)]
    pub start_frame: i32,
    #[serde(rename = "endFrame", default)]
    pub end_frame: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportRequest {
    #[serde(rename = "output_ext", default = "default_ext")]
    pub output_ext: String,
    #[serde(rename = "forward_up_axes", default = "default_axes")]
    pub forward_up_axes: String,
    #[serde(rename = "unit_size", default = "default_unit")]
    pub source_unit_size_text: String,
    #[serde(default = "default_scale")]
    pub scale: f32,
    #[serde(default)]
    pub materials: Vec<MaterialInfo>,
    #[serde(default)]
    pub animation: AnimationSettings,
    #[serde(skip)]
    pub raw_json_data: Vec<u8>,
}

fn default_ext() -> String {
    "cgf".to_string()
}
fn default_axes() -> String {
    "-Y+Z".to_string()
}
fn default_unit() -> String {
    "cm".to_string()
}
fn default_scale() -> f32 {
    1.0
}

impl Default for ImportRequest {
    fn default() -> Self {
        Self {
            output_ext: default_ext(),
            forward_up_axes: default_axes(),
            source_unit_size_text: default_unit(),
            scale: default_scale(),
            materials: Vec::new(),
            animation: AnimationSettings::default(),
            raw_json_data: Vec::new(),
        }
    }
}

impl ImportRequest {
    pub fn load_from_file(path: &Path) -> Result<Self, String> {
        let data = fs::read(path).map_err(|e| format!("Failed to read ImportRequest: {}", e))?;
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            let mut req: ImportRequest =
                serde_json::from_slice(&data).map_err(|e| e.to_string())?;
            req.raw_json_data = data;
            Ok(req)
        } else {
            Ok(ImportRequest::default())
        }
    }
}

use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
pub struct NodeLodParam {
    #[serde(rename = "autoGenerate", default = "default_true")]
    pub auto_generate: bool,
    #[serde(rename = "lodCount", default = "default_lod_count")]
    pub lod_count: usize,
    #[serde(rename = "percent", default = "default_percent")]
    pub percent: f32,
}

fn default_true() -> bool {
    true
}
fn default_lod_count() -> usize {
    3
}
fn default_percent() -> f32 {
    0.5
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct AutoLodSettings {
    #[serde(default)]
    pub nodes: HashMap<String, NodeLodParam>,
}

impl AutoLodSettings {
    pub fn get_node_param(&self, node_name: &str) -> NodeLodParam {
        self.nodes.get(node_name).cloned().unwrap_or(NodeLodParam {
            auto_generate: true,
            lod_count: 3,
            percent: 0.5,
        })
    }
}
